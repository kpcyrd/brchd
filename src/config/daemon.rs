use crate::args::Args;
use crate::config::{self, ConfigFile};
use crate::errors::*;
use serde::{Serialize, Deserialize};
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
        Self::build(config, args)
    }

    fn build(config: ConfigFile, _args: &Args) -> Result<DaemonConfig> {
        let socket = config::build_socket_path(config.daemon.socket, false)?;

        let mut destination = config.daemon.destination
            .ok_or_else(|| format_err!("destination is required"))?;

        if let Some(alias) = config::resolve_alias(&config.destinations, &destination)? {
            destination = alias.destination.to_string();
        }

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
        let config = DaemonConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
        });
    }

    #[test]
    fn daemon_resolve_alias() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "@home"

[destinations.home]
destination = "http://127.0.0.1:7070"
        "#).unwrap();
        let config = DaemonConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
        });
    }

    #[test]
    fn daemon_resolve_alias_arg() {
        let args = Args {
            destination: Some("@home".to_string()),
            ..Default::default()
        };
        let mut config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"

[destinations.home]
destination = "http://127.0.0.1:7070"
        "#).unwrap();
        config.update(&args);

        let config = DaemonConfig::build(config, &args).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
        });
    }

    #[test]
    fn daemon_invalid_alias() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "@home"
        "#).unwrap();
        let r = DaemonConfig::build(config, &Args::default());
        assert!(r.is_err());
    }
}
