use crate::args::Args;
use crate::config::{self, ConfigFile};
use crate::errors::*;
use serde::{Serialize, Deserialize};
use std::net::SocketAddr;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct UploadConfig {
    pub bind_addr: SocketAddr,
    pub destination: String,
    pub path_format: String,
}

impl UploadConfig {
    pub fn load(args: &Args) -> Result<UploadConfig> {
        let config = ConfigFile::load(args)?;
        Self::build(config, args)
    }

    fn build(config: ConfigFile, _args: &Args) -> Result<UploadConfig> {
        let destination = config.http.destination
            .ok_or_else(|| format_err!("destination is required"))?;

        let bind_addr = config.http.bind_addr
            .unwrap_or_else(config::default_port);

        let path_format = config.http.path_format
            .unwrap_or_else(|| config::DEFAULT_PATH_FORMAT.to_string());

        Ok(UploadConfig {
            destination,
            bind_addr,
            path_format,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_upload_config() {
        let config = ConfigFile::load_slice(br#"
[http]
destination = "/drop"
"#).unwrap();
        let config = UploadConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, UploadConfig {
            bind_addr: "[::]:7070".parse().unwrap(),
            destination: "/drop".to_string(),
            path_format: "%p".to_string(),
        });
    }
}
