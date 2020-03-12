use bufstream::BufStream;
use crate::args::Args;
use crate::errors::*;
use crate::ipc::{self, IpcMessage, Status};
use crate::queue::Item;
use crossbeam_channel::{self as channel, select};
use std::collections::VecDeque;
use std::fs;
use std::os::unix::net::{UnixStream, UnixListener};
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
enum Command {
    Subscribe(channel::Sender<IpcMessage>),
    PopQueue(channel::Sender<Item>),
    PushQueue(Item),
}

struct Server {
    rx: channel::Receiver<Command>,
    queue: VecDeque<Item>,
    total_workers: usize,
    idle_workers: VecDeque<channel::Sender<Item>>,
    subscribers: Vec<channel::Sender<IpcMessage>>,
    status: Status,
}

impl Server {
    fn new(rx: channel::Receiver<Command>, total_workers: usize) -> Server {
        Server {
            rx,
            queue: VecDeque::new(),
            total_workers,
            idle_workers: VecDeque::new(),
            subscribers: Vec::new(),
            status: Status::default(),
        }
    }

    fn add_subscriber(&mut self, tx: channel::Sender<IpcMessage>) {
        info!("adding new subscriber");
        tx.send(IpcMessage::StatusResp(self.status.clone())).ok();
        self.subscribers.push(tx);
    }

    fn ping_subscribers(&mut self) {
        trace!("pinging all subscribers");
        self.broadcast_subscribers(&IpcMessage::Ping);
    }

    fn update_status(&mut self) {
        let status = Status {
            idle_workers: self.idle_workers.len(),
            total_workers: self.total_workers,
            queue: self.queue.len(),
        };
        self.broadcast_subscribers(&IpcMessage::StatusResp(status.clone()));
        self.status = status;
    }

    fn broadcast_subscribers(&mut self, msg: &IpcMessage) {
        let before = self.subscribers.len();
        self.subscribers.retain(|c| c.send(msg.clone()).is_ok());
        let after = self.subscribers.len();

        if before > after {
            info!("disconnected {} subscribers", before - after);
        }
    }

    fn pop_queue(&mut self, worker: channel::Sender<Item>) {
        if let Some(task) = self.queue.pop_front() {
            debug!("assigning task to worker: {:?}", task);
            worker.send(task).expect("worker thread died");
        } else {
            debug!("parking worker thread as idle");
            self.idle_workers.push_back(worker);
        }
        self.update_status();
    }

    fn push_work(&mut self, task: Item) {
        if let Some(worker) = self.idle_workers.pop_front() {
            debug!("assigning task to worker: {:?}", task);
            worker.send(task).expect("worker thread died");
        } else {
            debug!("adding task to queue");
            self.queue.push_back(task);
        }
        self.update_status();
    }

    fn run(&mut self) {
        loop {
            select! {
                recv(self.rx) -> msg => {
                    debug!("received from channel: {:?}", msg);
                    match msg {
                        Ok(Command::Subscribe(tx)) => self.add_subscriber(tx),
                        Ok(Command::PopQueue(tx)) => self.pop_queue(tx),
                        Ok(Command::PushQueue(item)) => self.push_work(item),
                        Err(_) => break,
                    }
                }
                default(Duration::from_secs(60)) => self.ping_subscribers(),
            }
        }
    }
}

struct Worker {
    tx: channel::Sender<Command>,
}

impl Worker {
    fn new(tx: channel::Sender<Command>) -> Worker {
        Worker {
            tx,
        }
    }

    fn run(&mut self) {
        // TODO: lots of smart logic missing here
        loop {
            let (tx, rx) = channel::unbounded();
            self.tx.send(Command::PopQueue(tx)).unwrap();
            let task = rx.recv().unwrap();
            println!("working hard on task: {:?}", task);
        }
    }
}

struct Client {
    stream: BufStream<UnixStream>,
    tx: channel::Sender<Command>,
}

impl Client {
    fn new(tx: channel::Sender<Command>, stream: UnixStream) -> Client {
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
                IpcMessage::StatusReq => todo!("status request"),
                IpcMessage::StatusResp(_) => bail!("Unexpected ipc message"),
                IpcMessage::Queue(item) => {
                    self.write_server(Command::PushQueue(item));
                },
            }
        }
        info!("ipc client disconnected");
        Ok(())
    }
}

fn accept(tx: channel::Sender<Command>, stream: UnixStream) {
    info!("accepted ipc connection");
    let mut client = Client::new(tx, stream);
    if let Err(err) = client.run() {
        error!("ipc connection failed: {}", err);
    }
}

pub fn run(args: &Args) -> Result<()> {
    let path = Path::new(&args.socket);
    if path.exists() {
        fs::remove_file(&path)?;
    }

    // TODO: ensure parent folder exists

    let total_workers = 2;

    let (tx, rx) = channel::unbounded();
    for _ in 0..total_workers {
        let tx = tx.clone();
        thread::spawn(|| {
            let mut worker = Worker::new(tx);
            worker.run();
        });
    }

    thread::spawn(move || {
        let mut server = Server::new(rx, total_workers);
        server.run();
    });

    let listener = UnixListener::bind(&path)?;
    info!("ready to accept connections");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let tx = tx.clone();
                thread::spawn(|| accept(tx, stream));
            },
            Err(_err) => {
                break;
            }
        }
    }
    Ok(())
}