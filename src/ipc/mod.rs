use bufstream::BufStream;
use crate::errors::*;
use crate::queue::{Task, QueueClient};
use crate::status::Status;
use serde::{Serialize, Deserialize};
use std::io::prelude::*;

#[cfg(unix)]
#[path="unix.rs"]
mod os;

#[cfg(windows)]
#[path="windows.rs"]
mod os;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    Ping,
    Subscribe,
    StatusResp(Status),
    QueueReq,
    QueueResp(Vec<Task>),
    PushQueue(Task),
    Shutdown,
}

pub fn read<S: Read + Write>(stream: &mut BufStream<S>) -> Result<Option<IpcMessage>> {
    let mut buf = String::new();
    let n = stream.read_line(&mut buf)?;
    if n > 0 {
        let msg = serde_json::from_str(&buf[..n])?;
        debug!("received from ipc: {:?}", msg);
        Ok(Some(msg))
    } else {
        Ok(None)
    }
}

pub fn write<S: Read + Write>(stream: &mut BufStream<S>, msg: &IpcMessage) -> Result<()> {
    debug!("sending to ipc: {:?}", msg);
    let mut buf = serde_json::to_string(msg)?;
    buf.push('\n');
    stream.write_all(buf.as_bytes())?;
    stream.flush()?;
    Ok(())
}

pub use self::os::{IpcClient, IpcServer, build_socket_path};

impl QueueClient for IpcClient {
    fn push_work(&mut self, task: Task) -> Result<()> {
        info!("pushing task to daemon: {:?}", task);
        write(&mut self.stream, &IpcMessage::PushQueue(task))
    }
}

impl IpcClient {
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

    pub fn fetch_queue(&mut self) -> Result<Vec<Task>> {
        write(&mut self.stream, &IpcMessage::QueueReq)?;
        loop {
            return match read(&mut self.stream)? {
                Some(IpcMessage::Ping) => continue,
                Some(IpcMessage::QueueResp(queue)) => Ok(queue),
                Some(_) => bail!("Unexpected ipc message"),
                None => bail!("Daemon disconnected"),
            };
        }
    }
}
