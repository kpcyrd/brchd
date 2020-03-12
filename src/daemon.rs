use bufstream::BufStream;
use crate::args::Args;
use crate::errors::*;
use crate::ipc::{self, IpcMessage, Status};
use crate::queue::Item;
use std::collections::VecDeque;
use std::fs;
use std::os::unix::net::{UnixStream, UnixListener};
use std::path::Path;
use std::thread;
use std::sync::mpsc;

#[derive(Debug)]
enum Command {
    Subscribe(mpsc::Sender<Status>),
    PopQueue(mpsc::Sender<Item>),
    PushQueue(Item),
}

struct Server {
    rx: mpsc::Receiver<Command>,
    queue: VecDeque<Item>,
    idle_workers: VecDeque<mpsc::Sender<Item>>,
    subscribers: Vec<mpsc::Sender<Status>>,
    status: Status,
}

impl Server {
    fn new(rx: mpsc::Receiver<Command>) -> Server {
        Server {
            rx,
            queue: VecDeque::new(),
            idle_workers: VecDeque::new(),
            subscribers: Vec::new(),
            status: Status::default(),
        }
    }

    fn add_subscriber(&mut self, tx: mpsc::Sender<Status>) {
        info!("adding new subscriber");
        tx.send(self.status.clone()).ok();
        self.subscribers.push(tx);
    }

    fn update_status(&mut self) {
        let status = Status {
            idle_workers: self.idle_workers.len(),
            queue: self.queue.len(),
        };
        self.subscribers.retain(|c| c.send(status.clone()).is_ok());
        self.status = status;
    }

    fn pop_queue(&mut self, worker: mpsc::Sender<Item>) {
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
            if let Ok(msg) = self.rx.recv() {
                debug!("received from channel: {:?}", msg);
                match msg {
                    Command::Subscribe(tx) => self.add_subscriber(tx),
                    Command::PopQueue(tx) => self.pop_queue(tx),
                    Command::PushQueue(item) => self.push_work(item),
                }
            } else {
                break;
            }
        }
    }
}

struct Worker {
    tx: mpsc::Sender<Command>,
}

impl Worker {
    fn new(tx: mpsc::Sender<Command>) -> Worker {
        Worker {
            tx,
        }
    }

    fn run(&mut self) {
        // TODO: lots of smart logic missing here
        loop {
            let (tx, rx) = mpsc::channel();
            self.tx.send(Command::PopQueue(tx)).unwrap();
            let task = rx.recv().unwrap();
            println!("working hard on task: {:?}", task);
        }
    }
}

struct Client {
    stream: BufStream<UnixStream>,
    tx: mpsc::Sender<Command>,
}

impl Client {
    fn new(tx: mpsc::Sender<Command>, stream: UnixStream) -> Client {
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
        let (tx, rx) = mpsc::channel();
        self.write_server(Command::Subscribe(tx));

        for status in rx {
            self.write_line(&IpcMessage::StatusResp(status))?;
        }

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        while let Some(msg) = self.read_line()? {
            debug!("received from client: {:?}", msg);
            match msg {
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

fn accept(tx: mpsc::Sender<Command>, stream: UnixStream) {
    info!("accepted ipc connection");
    let mut client = Client::new(tx, stream);
    if let Err(err) = client.run() {
        error!("ipc connection failed: {}", err);
    }
}

pub fn run(_args: &Args) -> Result<()> {
    let path = Path::new("brchd.sock");
    if path.exists() {
        fs::remove_file(&path)?;
    }

    let (tx, rx) = mpsc::channel();
    for _ in 0..2 {
        let tx = tx.clone();
        thread::spawn(|| {
            let mut worker = Worker::new(tx);
            worker.run();
        });
    }

    thread::spawn(|| {
        let mut server = Server::new(rx);
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
