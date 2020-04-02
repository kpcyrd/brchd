use crate::args::Args;
use crate::config::{self, ConfigFile};
use crate::errors::*;
use serde::{Serialize, Deserialize};
// use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket: PathBuf,
    pub destination: String,
    pub concurrency: usize,
}

impl DaemonConfig {
    pub fn load(args: &Args) -> Result<DaemonConfig> {
        let config = ConfigFile::load(args)?;
        Self::build(config)
    }

    fn build(config: ConfigFile) -> Result<DaemonConfig> {
        let socket = config::build_socket_path(config.daemon.socket, false)?;

        let destination = config.daemon.destination
            .ok_or_else(|| format_err!("destination is required"))?;

        let concurrency = config.daemon.concurrency
            .unwrap_or(config::DEFAULT_CONCURRENCY);

        Ok(DaemonConfig {
            socket,
            destination,
            concurrency,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_daemon_config() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "http://127.0.0.1:7070"
"#).unwrap();
        let config = DaemonConfig::build(config).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
        });
    }
}
