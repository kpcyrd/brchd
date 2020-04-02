use crate::args::Args;
use crate::config::{self, ConfigFile};
use crate::errors::*;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub socket: PathBuf,
}

impl ClientConfig {
    pub fn load(args: &Args) -> Result<ClientConfig> {
        let config = ConfigFile::load(args)?;
        Self::build(config)
    }

    fn build(config: ConfigFile) -> Result<ClientConfig> {
        let socket = config::build_socket_path(config.daemon.socket, true)?;

        Ok(ClientConfig {
            socket,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_client_config() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
"#).unwrap();
        let config = ClientConfig::build(config).unwrap();
        assert_eq!(config, ClientConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
        });
    }
}
