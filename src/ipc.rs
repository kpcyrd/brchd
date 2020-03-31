use bufstream::BufStream;
use crate::errors::*;
use crate::queue::{Task, QueueClient};
use crate::status::Status;
use serde::{Serialize, Deserialize};
use std::io::prelude::*;
use std::os::unix::net::UnixStream;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    Ping,
    Subscribe,
    StatusReq,
    StatusResp(Status),
    Queue(Task),
}

pub fn read(stream: &mut BufStream<UnixStream>) -> Result<Option<IpcMessage>> {
    let mut buf = String::new();
    let n = stream.read_line(&mut buf)?;
    if n > 0 {
        let msg = serde_json::from_str(&buf[..n])?;
        Ok(Some(msg))
    } else {
        Ok(None)
    }
}

pub fn write(stream: &mut BufStream<UnixStream>, msg: &IpcMessage) -> Result<()> {
    let mut buf = serde_json::to_string(msg)?;
    buf.push('\n');
    stream.write_all(buf.as_bytes())?;
    stream.flush()?;
    Ok(())
}

pub struct IpcClient {
    stream: BufStream<UnixStream>,
}

impl QueueClient for IpcClient {
    fn push_work(&mut self, task: Task) -> Result<()> {
        info!("pushing task to daemon: {:?}", task);
        write(&mut self.stream, &IpcMessage::Queue(task))
    }
}

impl IpcClient {
    pub fn connect(path: &Path) -> Result<IpcClient> {
        debug!("connecting to {:?}", path);
        let stream = UnixStream::connect(path)
            .context("Failed to connect to brchd socket, is brchd -D running?")?;
        let stream = BufStream::new(stream);
        Ok(IpcClient {
            stream,
        })
    }

    pub fn subscribe(&mut self) -> Result<()> {
        write(&mut self.stream, &IpcMessage::Subscribe)
    }

    pub fn read_status(&mut self) -> Result<Option<Status>> {
        loop {
            return match read(&mut self.stream)? {
                Some(IpcMessage::Ping) => continue,
                Some(IpcMessage::StatusResp(status)) => Ok(Some(status)),
                Some(_) => bail!("Unexpected ipc message"),
                None => Ok(None),
            };
        }
    }
}
