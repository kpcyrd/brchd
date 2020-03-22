use crate::args::Args;
use crate::errors::*;
use serde::{Serialize, Deserialize};
use std::fs;
use std::net::{SocketAddr, IpAddr, Ipv6Addr};
use std::path::Path;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigFile {
    upload_dest: Option<String>,
    bind_addr: Option<SocketAddr>,
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
            self.upload_dest = Some(v.clone());
        }

        if let Some(v) = &args.bind_addr {
            self.bind_addr = Some(v.clone());
        }
    }

    pub fn build_upload_config(self) -> Result<UploadConfig> {
        let upload_dest = self.upload_dest
            .ok_or_else(|| format_err!("upload_dest is required"))?;

        let bind_addr = self.bind_addr
            .unwrap_or_else(default_port);

        Ok(UploadConfig {
            upload_dest,
            bind_addr,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadConfig {
    pub upload_dest: String,
    pub bind_addr: SocketAddr,
}

impl UploadConfig {
    pub fn load(args: &Args) -> Result<UploadConfig> {
        // TODO: take path from args, else use common paths
        let path = Path::new("config.toml");

        let mut config = if path.exists() {
            ConfigFile::load_from(path)?
        } else {
            ConfigFile::default()
        };

        config.update(args);
        config.build_upload_config()
    }
}
