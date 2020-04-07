use crate::config::DaemonConfig;
use crate::daemon::{Server, Command};
use crate::errors::*;
use crate::ipc::IpcMessage;
use crate::queue::{Task, QueueClient};
use crate::status::StatusWriter;
use crate::uploader::Worker;
use crossbeam_channel::{self as channel, Sender, Receiver};
use std::thread;

pub struct Standalone {
    status_rx: Receiver<IpcMessage>,
    tx: Sender<Command>,
}

impl QueueClient for Standalone {
    fn push_work(&mut self, task: Task) -> Result<()> {
        info!("pushing to queue: {:?}", task);
        self.tx.send(Command::PushQueue(task))?;
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.tx.send(Command::Shutdown)?;

        let mut w = StatusWriter::new();
        for msg in &self.status_rx {
            match msg {
                IpcMessage::StatusResp(status) => {
                    w.write(&status)?;
                },
                _ => (),
            }
        }
        w.finish()?;

        Ok(())
    }
}

impl Standalone {
    pub fn new(config: DaemonConfig) -> Result<Standalone> {
        let total_workers = config.concurrency;
        let (tx, rx) = channel::unbounded();
        for _ in 0..total_workers {
            let tx = tx.clone();
            let mut worker = Worker::new(tx,
                                         config.destination.clone(),
                                         config.path_format.clone(),
                                         config.proxy.clone(),
                                         config.pubkey.clone(),
                                         config.seckey.clone())
                .context("Failed to create worker")?;
            thread::spawn(move || {
                worker.run();
            });
        }

        let (status_tx, status_rx) = channel::unbounded();

        thread::spawn(move || {
            let mut server = Server::new(rx, total_workers);
            server.add_subscriber(status_tx);
            server.run();
        });

        Ok(Standalone {
            status_rx,
            tx,
        })
    }
}
