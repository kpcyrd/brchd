use bufstream::BufStream;
use crate::args::Args;
use crate::config::DaemonConfig;
use crate::errors::*;
use crate::ipc::{self, IpcServer, IpcMessage};
use crate::uploader::Worker;
use crate::queue::Task;
use crate::status::{Status, ProgressUpdate};
use crossbeam_channel::{self as channel, select};
use std::collections::VecDeque;
use std::io::prelude::*;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum Command {
    Subscribe(channel::Sender<IpcMessage>),
    PopQueue(channel::Sender<Task>),
    PushQueue(Task),
    FetchQueue(channel::Sender<Vec<Task>>),
    ProgressUpdate(ProgressUpdate),
    Shutdown,
}

pub struct Server {
    rx: channel::Receiver<Command>,
    queue: VecDeque<Task>,
    queue_size: u64,
    total_workers: usize,
    idle_workers: VecDeque<channel::Sender<Task>>,
    subscribers: Vec<channel::Sender<IpcMessage>>,
    status: Status,
    shutdown: bool,
}

impl Server {
    pub fn new(rx: channel::Receiver<Command>, total_workers: usize) -> Server {
        Server {
            rx,
            queue: VecDeque::new(),
            queue_size: 0,
            total_workers,
            idle_workers: VecDeque::new(),
            subscribers: Vec::new(),
            status: Status::default(),
            shutdown: false,
        }
    }

    pub fn add_subscriber(&mut self, tx: channel::Sender<IpcMessage>) {
        debug!("adding new subscriber");
        tx.send(IpcMessage::StatusResp(self.status.clone())).ok();
        self.subscribers.push(tx);
    }

    fn ping_subscribers(&mut self) {
        trace!("pinging all subscribers");
        self.broadcast_subscribers(&IpcMessage::Ping);
    }

    fn update_progress(&mut self, update: ProgressUpdate) {
        self.status.update(update);
        self.broadcast_subscribers(&IpcMessage::StatusResp(self.status.clone()));
    }

    fn update_stats(&mut self) {
        self.status.idle_workers = self.idle_workers.len();
        self.status.total_workers = self.total_workers;
        self.status.queue = self.queue.len();
        self.status.queue_size = self.queue_size;
        self.broadcast_subscribers(&IpcMessage::StatusResp(self.status.clone()));
    }

    fn broadcast_subscribers(&mut self, msg: &IpcMessage) {
        let before = self.subscribers.len();
        self.subscribers.retain(|c| c.send(msg.clone()).is_ok());
        let after = self.subscribers.len();

        if before > after {
            debug!("disconnected {} subscribers", before - after);
        }
    }

    fn pop_queue(&mut self, worker: channel::Sender<Task>) -> bool {
        if let Some(task) = self.queue.pop_front() {
            self.queue_size -= task.size;
            debug!("assigning task to worker: {:?}", task);
            worker.send(task).expect("worker thread died");
        } else {
            debug!("parking worker thread as idle");
            self.idle_workers.push_back(worker);
        }
        self.update_stats();

        // check if we are supposed to shutdown the server
        self.shutdown && self.queue.is_empty() && self.idle_workers.len() == self.total_workers
    }

    fn push_work(&mut self, task: Task) {
        if let Some(worker) = self.idle_workers.pop_front() {
            debug!("assigning task to worker: {:?}", task);
            worker.send(task).expect("worker thread died");
        } else {
            debug!("adding task to queue");
            self.queue_size += task.size;
            self.queue.push_back(task);
        }
        self.update_stats();
    }

    fn fetch_queue(&mut self, worker: channel::Sender<Vec<Task>>) {
        let queue = self.queue.iter().cloned().collect();
        worker.send(queue).expect("worker thread died");
    }

    pub fn run(&mut self) {
        loop {
            select! {
                recv(self.rx) -> msg => {
                    debug!("received from channel: {:?}", msg);
                    match msg {
                        Ok(Command::Subscribe(tx)) => self.add_subscriber(tx),
                        Ok(Command::PopQueue(tx)) => if self.pop_queue(tx) {
                            break;
                        },
                        Ok(Command::PushQueue(task)) => self.push_work(task),
                        Ok(Command::FetchQueue(tx)) => self.fetch_queue(tx),
                        Ok(Command::ProgressUpdate(update)) => self.update_progress(update),
                        Ok(Command::Shutdown) => self.shutdown = true,
                        Err(_) => break,
                    }
                }
                default(Duration::from_secs(60)) => self.ping_subscribers(),
            }
        }
    }
}

struct Client<S: Read + Write> {
    stream: BufStream<S>,
    tx: channel::Sender<Command>,
}

impl<S: Read + Write> Client<S> {
    fn new(tx: channel::Sender<Command>, stream: S) -> Client<S> {
        let stream = BufStream::new(stream);
        Client {
            stream,
            tx,
        }
    }

    #[inline]
    fn read_line(&mut self) -> Result<Option<IpcMessage>> {
        ipc::read(&mut self.stream)
    }

    #[inline]
    fn write_line(&mut self, msg: &IpcMessage) -> Result<()> {
        ipc::write(&mut self.stream, msg)
    }

    fn write_server(&self, cmd: Command) {
        self.tx.send(cmd).unwrap();
    }

    fn subscribe_loop(&mut self) -> Result<()> {
        let (tx, rx) = channel::unbounded();
        self.write_server(Command::Subscribe(tx));

        for msg in rx {
            if self.write_line(&msg).is_err() {
                break;
            }
        }

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        while let Some(msg) = self.read_line()? {
            debug!("received from client: {:?}", msg);
            match msg {
                IpcMessage::Ping => bail!("Unexpected ipc message"),
                IpcMessage::Subscribe => self.subscribe_loop()?,
                IpcMessage::StatusResp(_) => bail!("Unexpected ipc message"),

                IpcMessage::QueueReq => {
                    let (tx, rx) = channel::unbounded();
                    self.write_server(Command::FetchQueue(tx));
                    let queue = rx.recv().unwrap();
                    let msg = IpcMessage::QueueResp(queue);
                    if self.write_line(&msg).is_err() {
                        break;
                    }
                },
                IpcMessage::QueueResp(_) => bail!("Unexpected ipc message"),

                IpcMessage::PushQueue(task) => {
                    self.write_server(Command::PushQueue(task));
                },

                IpcMessage::Shutdown => bail!("Unexpected ipc message"),
            }
        }
        debug!("ipc client disconnected");
        Ok(())
    }
}

fn accept<S: Read + Write>(tx: channel::Sender<Command>, stream: S) {
    debug!("accepted ipc connection");
    let mut client = Client::new(tx, stream);
    if let Err(err) = client.run() {
        error!("ipc connection failed: {}", err);
    }
}

pub fn run(args: &Args) -> Result<()> {
    let config = DaemonConfig::load(&args)?;
    let listener = IpcServer::bind(&config.socket)
        .context("Failed to create ipc server")?;

    let total_workers = config.concurrency;
    let (tx, rx) = channel::unbounded();
    for _ in 0..total_workers {
        let tx = tx.clone();
        let mut worker = Worker::new(tx,
                                     config.destination.clone(),
                                     config.path_format.clone(),
                                     config.proxy.clone(),
                                     args.accept_invalid_certs,
                                     config.pubkey,
                                     config.seckey.clone())
            .context("Failed to create worker")?;
        thread::spawn(move || {
            worker.run();
        });
    }

    thread::spawn(move || {
        let mut server = Server::new(rx, total_workers);
        server.run();
    });

    info!("ready to accept connections");
    loop {
        let stream = listener.accept()
            .context("Failed to accept ipc client")?;
        let tx = tx.clone();
        thread::spawn(|| accept(tx, stream));
    }
}
