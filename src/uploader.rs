use crate::crypto::{PublicKey, SecretKey};
use crate::crypto::upload::EncryptedUpload;
use crate::daemon::Command;
use crate::destination;
use crate::errors::*;
use crate::queue::{Task, Target, PathTarget};
use crate::pathspec::UploadContext;
use crate::status::{ProgressUpdate, UploadStart, UploadProgress, UploadEnd};
use crossbeam_channel::{self as channel};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use reqwest::Proxy;
use reqwest::blocking::{Client, multipart};
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use std::time::{Instant, Duration};
use url::Url;

const UPDATE_NOTIFY_RATELIMIT: Duration = Duration::from_millis(250);

pub struct CryptoConfig {
    pubkey: PublicKey,
    seckey: Option<SecretKey>,
}

pub struct Worker {
    client: Client,
    destination: Destination,
    path_format: String,
    tx: channel::Sender<Command>,
    crypto: Option<CryptoConfig>,
}

pub enum Destination {
    Path(String),
    Url(Url),
}

impl Worker {
    pub fn new(tx: channel::Sender<Command>, destination: String, path_format: String, proxy: Option<String>, pubkey: Option<PublicKey>, seckey: Option<SecretKey>) -> Result<Worker> {
        let destination = if let Ok(url) = destination.parse::<Url>() {
            Destination::Url(url)
        } else {
            Destination::Path(destination)
        };

        let mut builder = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(None);
        if let Some(proxy) = &proxy {
            builder = builder.proxy(Proxy::all(proxy)?);
        }
        let client = builder.build()?;

        let crypto = pubkey.map(|pubkey| {
            CryptoConfig {
                pubkey,
                seckey,
            }
        });

        Ok(Worker {
            client,
            destination,
            path_format,
            tx,
            crypto,
        })
    }

    fn pop_work(&self) -> Option<Task> {
        let (tx, rx) = channel::unbounded();
        self.tx.send(Command::PopQueue(tx)).unwrap();
        rx.recv().ok()
    }

    pub fn run(&mut self) {
        // TODO: lots of smart logic missing here
        while let Some(task) = self.pop_work() {
            info!("starting task: {:?}", task);
            let (path, result) = match task.target {
                Target::Path(PathTarget {
                    path,
                    resolved,
                }) => {
                    (format!("{:?}", path), self.start_upload(path, resolved))
                },
                Target::Url(_url) => todo!("url task"),
            };

            if let Err(err) = result {
                // TODO: consider retry
                // TODO: notify the monitor somehow(?)
                error!("upload failed ({}): {}", path, err);
            }
        }
    }

    pub fn start_upload(&self, path: PathBuf, resolved: PathBuf) -> Result<()> {
        // TODO: this works for now, but we need to revisit this
        // TODO: this doesn't remove /../ inside the path
        let mut path = path.to_string_lossy().into_owned();
        let label = path.clone();
        while path.starts_with("../") {
            path = path[3..].to_string();
        }

        let file = File::open(&resolved)?;
        let metadata = fs::metadata(&resolved)?;
        let total = metadata.len();

        // TODO: instead of boxing, we could refactor this into generics (benchmark this)
        let (file, total) = if let Some(crypto) = &self.crypto {
            let file = EncryptedUpload::new(file, &crypto.pubkey, crypto.seckey.as_ref())?;
            let total = file.total_with_overhead(total);
            (Box::new(file) as Box<dyn Read + Send>, total)
        } else {
            (Box::new(file) as Box<dyn Read + Send>, total)
        };

        let (upload, id) = Upload::new(self.tx.clone(), file);
        notify(&self.tx, ProgressUpdate::UploadStart(UploadStart {
            id: id.clone(),
            label,
            total,
        }));

        let result = match &self.destination {
            Destination::Path(destination) => self.copy_file(upload, destination.clone(), &path, resolved),
            Destination::Url(url) => self.upload_file(url.clone(), upload, path, total),
        };
        notify(&self.tx, ProgressUpdate::UploadEnd(UploadEnd {
            id,
        }));

        result
    }

    fn copy_file(&self, mut upload: Upload, destination: String, path: &str, full_path: PathBuf) -> Result<()> {
        let full_path = full_path.to_string_lossy(); // TODO: is to_string_lossy the right approach here?
        let full_path = full_path.trim_start_matches('/').to_string();

        destination::save_sync(&mut upload, UploadContext::new(
            destination,
            self.path_format.clone(),
            None, // TODO: in case of an url, set this to the host
            path,
            Some(full_path),
        )?)
    }

    fn upload_file(&self, url: Url, upload: Upload, path: String, total: u64) -> Result<()> {
        let file = multipart::Part::reader_with_length(upload, total)
            .file_name(path)
            .mime_str("application/octet-stream")?;

        let form = multipart::Form::new()
            .part("file", file);

        info!("uploading to {:?}", url);
        let resp = self.client
            .post(url)
            .multipart(form)
            .send()?;

        info!("uploaded: {:?}", resp);
        let body = resp.text()?;
        info!("uploaded(text): {:?}", body);

        Ok(())
    }
}

fn random_id() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .collect()
}

struct Upload {
    id: String,
    tx: channel::Sender<Command>,
    inner: Box<dyn Read + Send>,
    bytes_read: u64,
    started: Instant, // TODO: sample recent upload speed instead of total
    last_update: Instant,
}

impl Upload {
    fn new(tx: channel::Sender<Command>, inner: Box<dyn Read + Send>) -> (Upload, String) {
        let id = random_id();
        let now = Instant::now();
        (Upload {
            id: id.clone(),
            tx,
            inner,
            bytes_read: 0,
            started: now,
            last_update: now,
        }, id)
    }

    fn notify(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_update) >= UPDATE_NOTIFY_RATELIMIT {
            let secs_elapsed = self.started.elapsed().as_secs();
            let speed = if secs_elapsed > 0 {
                self.bytes_read / secs_elapsed
            } else {
                self.bytes_read
            };
            notify(&self.tx, ProgressUpdate::UploadProgress(UploadProgress {
                id: self.id.clone(),
                bytes_read: self.bytes_read,
                speed,
            }));

            self.last_update = now;
        }
    }
}

fn notify(tx: &channel::Sender<Command>, update: ProgressUpdate) {
    tx.send(Command::ProgressUpdate(update)).unwrap();
}

impl Read for Upload {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
            .map(|n| {
                self.bytes_read += n as u64;
                self.notify();
                n
            })
    }
}
