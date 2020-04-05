use bufstream::BufStream;
use crate::errors::*;
use std::fs;
use std::os::unix::net::{UnixStream, UnixListener};
use std::path::{Path, PathBuf};

pub fn build_socket_path(socket: Option<PathBuf>, search: bool) -> Result<PathBuf> {
    if let Some(path) = socket {
        Ok(path)
    } else {
        let path = dirs::data_dir()
            .ok_or_else(|| format_err!("Failed to find data directory"))?;
        let path = path.join("brchd.sock");
        if !search || path.exists() {
            return Ok(path);
        }

        let path = PathBuf::from("/var/run/brchd/sock");
        if path.exists() {
            return Ok(path);
        }

        bail!("Could not find brchd socket, is brchd -D running?")
    }
}

pub struct IpcClient {
    pub stream: BufStream<UnixStream>,
}

impl IpcClient {
    pub fn connect(path: &Path) -> Result<IpcClient> {
        debug!("connecting to {:?}", path);
        let stream = UnixStream::connect(path)
            .context("Failed to connect to brchd socket, is brchd -D running?")?;
        debug!("connected");
        let stream = BufStream::new(stream);
        Ok(IpcClient {
            stream,
        })
    }
}

pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    pub fn bind(path: &Path) -> Result<IpcServer> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create socket parent folder")?;
        }

        if path.exists() {
            fs::remove_file(&path)
                .context("Failed to remove old socket")?;
        }

        let listener = UnixListener::bind(&path)
            .context("Failed to bind to socket path")?;

        Ok(IpcServer { listener })
    }

    pub fn accept(&self) -> Result<UnixStream> {
        let (stream, _) = self.listener.accept()?;
        Ok(stream)
    }
}
