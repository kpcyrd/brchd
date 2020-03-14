use console::Term;
use crate::errors::*;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::io::prelude::*;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Status {
    pub idle_workers: usize,
    pub total_workers: usize,
    pub queue: usize,
    pub progress: BTreeMap<String, Progress>,
}

impl Status {
    pub fn update(&mut self, update: ProgressUpdate) {
        match update {
            ProgressUpdate::UploadStart(start) => {
                self.progress.insert(start.key, Progress {
                    bytes_read: 0,
                    total: start.total,
                });
            },
            ProgressUpdate::UploadProgress(progress) => {
                let p = self.progress.get_mut(&progress.key).expect(&format!("progress bar not found: {:?}", progress.key));
                p.bytes_read = progress.bytes_read;
            },
            ProgressUpdate::UploadEnd(end) => {
                self.progress.remove(&end.key);
            },
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Progress {
    bytes_read: u64,
    total: u64,
}

#[derive(Debug)]
pub enum ProgressUpdate {
    UploadStart(UploadStart),
    UploadProgress(UploadProgress),
    UploadEnd(UploadEnd),
}

#[derive(Debug)]
pub struct UploadStart {
    pub key: String,
    pub total: u64,
}

#[derive(Debug)]
pub struct UploadProgress {
    pub key: String,
    pub bytes_read: u64,
}

#[derive(Debug)]
pub struct UploadEnd {
    pub key: String,
}

pub struct StatusWriter {
    term: Term,
    height: usize,
}

impl StatusWriter {
    pub fn new() -> StatusWriter {
        StatusWriter{
            term: Term::stderr(),
            height: 0,
        }
    }

    pub fn write(&mut self, status: Status) -> Result<()> {
        // TODO: check if total is 0
        // println!("Upload progress: {}/{} bytes ({}%)", self.bytes_read, self.total, self.bytes_read * 100 / self.total);

        self.term.clear_line()?;
        if  self.height > 0 {
            self.term.clear_last_lines(self.height)?;
        }
        self.height = 0;

        for p in status.progress.values() {
            writeln!(self.term, "{:?}", p)?;
            self.height += 1;
        }

        write!(self.term, " :: idle_workers={}, total_workers={}, queue={}\r",
            status.idle_workers,
            status.total_workers,
            status.queue,
        )?;

        Ok(())
    }
}
