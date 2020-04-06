use crate::args::Args;
use crate::errors::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub daemon: Daemon,
    #[serde(default)]
    pub http: Http,
    #[serde(default)]
    pub crypto: Crypto,
    #[serde(default)]
    pub destinations: HashMap<String, Destination>,
    #[serde(default)]
    pub pubkeys: HashMap<String, Pubkey>,
}

impl ConfigFile {
    fn load_from<P: AsRef<Path>>(path: P) -> Result<ConfigFile> {
        let buf = fs::read(path)
            .context("Failed to read config file")?;
        ConfigFile::load_slice(&buf)
    }

    #[inline]
    pub fn load_slice(buf: &[u8]) -> Result<ConfigFile> {
        toml::from_slice(&buf)
            .context("Failed to parse config file")
            .map_err(Error::from)
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

        if let Some(v) = &args.proxy {
            self.daemon.proxy = Some(v.clone());
        }

        if let Some(v) = &args.path_format {
            self.http.path_format = Some(v.clone());
        }

        if let Some(v) = &args.pubkey {
            self.crypto.pubkey = Some(v.clone());
            self.daemon.pubkey = Some(v.clone());
        }

        if let Some(v) = &args.seckey {
            self.crypto.seckey = Some(v.clone());
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
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Daemon {
    pub socket: Option<PathBuf>,
    pub destination: Option<String>,
    pub concurrency: Option<usize>,
    pub pubkey: Option<String>,
    pub proxy: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Http {
    pub bind_addr: Option<SocketAddr>,
    pub destination: Option<String>,
    pub path_format: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Crypto {
    pub pubkey: Option<String>,
    pub seckey: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Destination {
    pub destination: String,
    pub pubkey: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pubkey {
    pub pubkey: String,
}
