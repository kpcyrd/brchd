use crate::daemon::Command;
use crate::errors::*;
use crate::queue::Item;
use crate::status::{ProgressUpdate, UploadStart, UploadProgress, UploadEnd};
use crossbeam_channel::{self as channel};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use reqwest::blocking::{Client, multipart};
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::time::Duration;

pub struct Worker {
    tx: channel::Sender<Command>,
}

impl Worker {
    pub fn new(tx: channel::Sender<Command>) -> Worker {
        Worker {
            tx,
        }
    }

    pub fn run(&mut self) {
        // TODO: lots of smart logic missing here
        loop {
            let (tx, rx) = channel::unbounded();
            self.tx.send(Command::PopQueue(tx)).unwrap();
            let task = rx.recv().unwrap();

            info!("starting task: {:?}", task);
            let result = match task {
                Item::Path(path) => {
                    self.start_upload(&path)
                },
                Item::Url(_url) => todo!("url item"),
            };

            if let Err(err) = result {
                // TODO: consider retry
                // TODO: notify the monitor somehow(?)
                error!("upload failed: {}", err);
            }
        }
    }

    pub fn start_upload(&self, path: &Path) -> Result<()> {
        let file = File::open(&path)?;
        let metadata = fs::metadata(&path)?;
        let total = metadata.len();

        let (upload, key) = Upload::new(self.tx.clone(), file);

        notify_start(&self.tx, key.clone(), total);
        let result = self.upload_file(upload, path, total);
        notify_end(&self.tx, key.clone());

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

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()?;

        let resp = client
            .post("http://localhost:7070/")
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
}

impl<R> Upload<R> {
    fn new(tx: channel::Sender<Command>, inner: R) -> (Upload<R>, String) {
        let key = random_id();
        (Upload {
            key: key.clone(),
            tx,
            inner,
            bytes_read: 0,
        }, key)
    }
}

fn notify(tx: &channel::Sender<Command>, update: ProgressUpdate) {
    tx.send(Command::ProgressUpdate(update)).unwrap();
}

fn notify_start(tx: &channel::Sender<Command>, key: String, total: u64) {
    notify(tx, ProgressUpdate::UploadStart(UploadStart {
        key,
        total,
    }));
}

fn notify_progress(tx: &channel::Sender<Command>, key: String, bytes_read: u64) {
    notify(tx, ProgressUpdate::UploadProgress(UploadProgress {
        key,
        bytes_read,
    }));
}

fn notify_end(tx: &channel::Sender<Command>, key: String) {
    notify(tx, ProgressUpdate::UploadEnd(UploadEnd {
        key,
    }));
}

// TODO: add a ratelimit for progress notifications
impl<R: Read> Read for Upload<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
            .map(|n| {
                self.bytes_read += n as u64;
                notify_progress(&self.tx, self.key.clone(), self.bytes_read);
                n
            })
    }
}
