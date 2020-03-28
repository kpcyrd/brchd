use crate::args::Args;
use crate::errors::*;
use serde::{Serialize, Deserialize};
use std::fs;
use std::net::{SocketAddr, IpAddr, Ipv6Addr};
use std::path::{Path, PathBuf};

const DEFAULT_CONCURRENCY: usize = 3;
const DEFAULT_PATH_FORMAT: &str = "%p";

fn find_config_file() -> Option<PathBuf> {
    if let Some(path) = dirs::config_dir() {
        let path = path.join("brchd.toml");
        if path.exists() {
            return Some(path);
        }
    }

    let path = PathBuf::from("/etc/brchd.toml");
    if path.exists() {
        return Some(path);
    }

    None
}

fn build_socket_path(socket: Option<PathBuf>, search: bool) -> Result<PathBuf> {
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

        bail!("Socket path not found")
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    daemon: Daemon,
    #[serde(default)]
    http: Http,
}

impl ConfigFile {
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<ConfigFile> {
        let buf = fs::read(path)
            .context("Failed to read config file")?;
        let config = toml::from_slice(&buf)
            .context("Failed to parse config file")?;
        Ok(config)
    }

    pub fn update(&mut self, args: &Args) {
        if let Some(v) = &args.destination {
            self.daemon.destination = Some(v.clone());
            self.http.destination = Some(v.clone());
        }

        if let Some(v) = &args.bind_addr {
            self.http.bind_addr = Some(*v);
        }

        if let Some(v) = args.concurrency {
            self.daemon.concurrency = Some(v);
        }

        if let Some(v) = &args.path_format {
            self.http.path_format = Some(v.clone());
        }
    }

    pub fn load(args: &Args) -> Result<ConfigFile> {
        let mut config = if let Some(path) = &args.config {
            ConfigFile::load_from(path)?
        } else if let Some(path) = find_config_file() {
            ConfigFile::load_from(path)?
        } else {
            ConfigFile::default()
        };

        config.update(args);
        Ok(config)
    }

    pub fn build_daemon_config(self) -> Result<DaemonConfig> {
        let socket = build_socket_path(self.daemon.socket, false)?;

        let destination = self.daemon.destination
            .ok_or_else(|| format_err!("destination is required"))?;

        let concurrency = self.daemon.concurrency
            .unwrap_or(DEFAULT_CONCURRENCY);

        Ok(DaemonConfig {
            socket,
            destination,
            concurrency,
        })
    }

    pub fn build_client_config(self) -> Result<ClientConfig> {
        let socket = build_socket_path(self.daemon.socket, true)?;

        Ok(ClientConfig {
            socket,
        })
    }

    pub fn build_upload_config(self) -> Result<UploadConfig> {
        let destination = self.http.destination
            .ok_or_else(|| format_err!("destination is required"))?;

        let bind_addr = self.http.bind_addr
            .unwrap_or_else(default_port);

        let path_format = self.http.path_format
            .unwrap_or_else(|| DEFAULT_PATH_FORMAT.to_string());

        Ok(UploadConfig {
            destination,
            bind_addr,
            path_format,
        })
    }
}

#[inline(always)]
fn default_port() -> SocketAddr {
    SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)),
        7070,
    )
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Daemon {
    socket: Option<PathBuf>,
    destination: Option<String>,
    concurrency: Option<usize>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Http {
    bind_addr: Option<SocketAddr>,
    destination: Option<String>,
    path_format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket: PathBuf,
    pub destination: String,
    pub concurrency: usize,
}

impl DaemonConfig {
    pub fn load(args: &Args) -> Result<DaemonConfig> {
        ConfigFile::load(args)?
            .build_daemon_config()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    pub socket: PathBuf,
}

impl ClientConfig {
    pub fn load(args: &Args) -> Result<ClientConfig> {
        ConfigFile::load(args)?
            .build_client_config()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadConfig {
    pub bind_addr: SocketAddr,
    pub destination: String,
    pub path_format: String,
}

impl UploadConfig {
    pub fn load(args: &Args) -> Result<UploadConfig> {
        ConfigFile::load(args)?
            .build_upload_config()
    }
}
