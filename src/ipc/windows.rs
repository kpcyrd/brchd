use bufstream::BufStream;
use crate::errors::*;
use named_pipe::{PipeOptions, PipeServer, PipeClient};
use std::path::{Path, PathBuf};

pub fn build_socket_path(socket: Option<PathBuf>, _search: bool) -> Result<PathBuf> {
    if let Some(path) = socket {
        Ok(path)
    } else {
        Ok(PathBuf::from(r"\\.\pipe\brchd"))
    }
}

pub struct IpcClient {
    pub stream: BufStream<PipeClient>,
}

impl IpcClient {
    pub fn connect(path: Option<PathBuf>) -> Result<IpcClient> {
        let path = build_socket_path(path, true)?;
        debug!("connecting to {:?}", path);
        let stream = PipeClient::connect(path)
            .context("Failed to connect to brchd socket, is brchd -D running?")?;
        debug!("connected");
        let stream = BufStream::new(stream);
        Ok(IpcClient {
            stream,
        })
    }
}

pub struct IpcServer {
    listener: PipeOptions,
}

impl IpcServer {
    pub fn bind(path: &Path) -> Result<IpcServer> {
        let mut listener = PipeOptions::new(path);
        listener.first(false);
        Ok(IpcServer { listener })
    }

    pub fn accept(&self) -> Result<PipeServer> {
        let stream = self.listener.single()
            .context("Failed to open named pipe")?;

        let stream = stream.wait()
            .context("Failed to wait for named pipe client")?;

        Ok(stream)
    }
}
