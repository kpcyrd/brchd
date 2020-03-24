use crate::daemon::Command;
use crate::errors::*;
use crate::queue::Target;
use crate::status::{ProgressUpdate, UploadStart, UploadProgress, UploadEnd};
use crossbeam_channel::{self as channel};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use reqwest::blocking::{Client, multipart};
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::time::{Instant, Duration};

const UPDATE_NOTIFY_RATELIMIT: Duration = Duration::from_millis(250);

pub struct Worker {
    client: Client,
    destination: String,
    tx: channel::Sender<Command>,
}

impl Worker {
    pub fn new(destination: String, tx: channel::Sender<Command>) -> Result<Worker> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(None)
            .build()?;

        Ok(Worker {
            client,
            destination,
            tx,
        })
    }

    pub fn run(&mut self) {
        // TODO: lots of smart logic missing here
        loop {
            let (tx, rx) = channel::unbounded();
            self.tx.send(Command::PopQueue(tx)).unwrap();
            let task = rx.recv().unwrap();

            info!("starting task: {:?}", task);
            let (path, result) = match task.target {
                Target::Path(path) => {
                    (format!("{:?}", path), self.start_upload(&path))
                },
                Target::Url(_url) => todo!("url item"),
            };

            if let Err(err) = result {
                // TODO: consider retry
                // TODO: notify the monitor somehow(?)
                error!("upload failed ({}): {}", path, err);
            }
        }
    }

    pub fn start_upload(&self, path: &Path) -> Result<()> {
        let file = File::open(&path)?;
        let metadata = fs::metadata(&path)?;
        let total = metadata.len();

        let (upload, key) = Upload::new(self.tx.clone(), file);

        notify(&self.tx, ProgressUpdate::UploadStart(UploadStart {
            key: key.clone(),
            label: path.to_string_lossy().into_owned(),
            total,
        }));
        let result = self.upload_file(upload, path, total);
        notify(&self.tx, ProgressUpdate::UploadEnd(UploadEnd {
            key,
        }));

        result
    }

    fn upload_file(&self, upload: Upload<File>, path: &Path, total: u64) -> Result<()> {
        let filename = path.file_name()
            .ok_or_else(|| format_err!("Could not figure out filename of path"))?
            .to_string_lossy()
            .into_owned();

        let file = multipart::Part::reader_with_length(upload, total)
            .file_name(filename) // TODO: if absolute path, truncate the first slash
            .mime_str("application/octet-stream")?;

        let form = multipart::Form::new()
            .part("file", file);

        info!("uploading to {:?}", self.destination);
        let resp = self.client
            .post(&self.destination)
            .multipart(form)
            .send()?;

        info!("uploaded: {:?}", resp);
        let body = resp.text();
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

struct Upload<R> {
    key: String,
    tx: channel::Sender<Command>,
    inner: R,
    bytes_read: u64,
    started: Instant, // TODO: sample recent upload speed instead of total
    last_update: Instant,
}

impl<R> Upload<R> {
    fn new(tx: channel::Sender<Command>, inner: R) -> (Upload<R>, String) {
        let key = random_id();
        let now = Instant::now();
        (Upload {
            key: key.clone(),
            tx,
            inner,
            bytes_read: 0,
            started: now,
            last_update: now,
        }, key)
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
                key: self.key.clone(),
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

impl<R: Read> Read for Upload<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
            .map(|n| {
                self.bytes_read += n as u64;
                self.notify();
                n
            })
    }
}
