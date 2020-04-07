use crate::args::Args;
use crate::config::{self, ConfigFile};
use crate::crypto;
use crate::errors::*;
use crate::ipc;
use serde::{Serialize, Deserialize};
use sodiumoxide::crypto::box_::{PublicKey, SecretKey};
use std::path::PathBuf;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket: PathBuf,
    pub destination: String,
    pub concurrency: usize,
    pub path_format: String,
    pub proxy: Option<String>,
    pub pubkey: Option<PublicKey>,
    pub seckey: Option<SecretKey>,
}

impl DaemonConfig {
    pub fn load(args: &Args) -> Result<DaemonConfig> {
        let config = ConfigFile::load(args)?;
        Self::build(config, args)
    }

    fn build(config: ConfigFile, args: &Args) -> Result<DaemonConfig> {
        let socket = ipc::build_socket_path(config.daemon.socket, false)?;

        let mut destination = config.daemon.destination
            .ok_or_else(|| format_err!("destination is required"))?;

        let mut pubkey = config.daemon.pubkey;
        let seckey = config.crypto.seckey;

        if let Some(alias) = config::resolve_alias(&config.destinations, &destination)? {
            destination = alias.destination.clone();
            if args.pubkey.is_none() && alias.pubkey.is_some() {
                pubkey = alias.pubkey.clone();
            }
        }

        let concurrency = config.daemon.concurrency
            .unwrap_or(config::DEFAULT_CONCURRENCY);

        let path_format = config.http.path_format
            .unwrap_or_else(|| config::DEFAULT_PATH_FORMAT.to_string());

        let pubkey = if let Some(pubkey) = pubkey {
            let pubkey = if let Some(alias) = config::resolve_alias(&config.pubkeys, &pubkey)? {
                &alias.pubkey
            } else {
                &pubkey
            };
            let pubkey = crypto::decode_pubkey(&pubkey)?;
            Some(pubkey)
        } else {
            None
        };

        let seckey = if let Some(seckey) = seckey {
            let seckey = crypto::decode_seckey(&seckey)?;
            Some(seckey)
        } else {
            None
        };

        Ok(DaemonConfig {
            socket,
            destination,
            concurrency,
            path_format,
            proxy: config.daemon.proxy,
            pubkey,
            seckey,
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
            proxy: None,
            pubkey: None,
            seckey: None,
        });
    }

    #[test]
    fn all_daemon_config() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "http://127.0.0.1:7070"
concurrency = 1
proxy = "socks5://127.0.0.1:9150"
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="

[crypto]
seckey = "5LYdSbVM3Pxnvzi71bZedjNXgnu0ZIjEObJeTqa3UAU="
"#).unwrap();
        let config = DaemonConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 1,
            proxy: Some("socks5://127.0.0.1:9150".to_string()),
            pubkey: Some(PublicKey::from_slice(&[
                115, 27, 214, 39, 98, 102, 27, 232, 92, 84, 12, 139, 20, 146,
                44, 161, 243, 112, 15, 176, 44, 198, 42, 22, 195, 238, 225,
                196, 48, 91, 122, 59,
            ]).unwrap()),
            seckey: Some(SecretKey::from_slice(&[
                228, 182, 29, 73, 181, 76, 220, 252, 103, 191, 56, 187, 213,
                182, 94, 118, 51, 87, 130, 123, 180, 100, 136, 196, 57, 178,
                94, 78, 166, 183, 80, 5,
            ]).unwrap()),
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
            proxy: None,
            pubkey: None,
            seckey: None,
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
            proxy: None,
            pubkey: None,
            seckey: None,
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

    #[test]
    fn daemon_without_pubkey_from_crypto_section() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "http://127.0.0.1:7070"

[crypto]
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="
        "#).unwrap();
        let config = DaemonConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
            proxy: None,
            pubkey: None,
            seckey: None,
        });

    }

    #[test]
    fn daemon_with_seckey_from_crypto_section() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "http://127.0.0.1:7070"

[crypto]
seckey = "5LYdSbVM3Pxnvzi71bZedjNXgnu0ZIjEObJeTqa3UAU="
        "#).unwrap();
        let config = DaemonConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
            proxy: None,
            pubkey: None,
            seckey: Some(SecretKey::from_slice(&[
                228, 182, 29, 73, 181, 76, 220, 252, 103, 191, 56, 187, 213,
                182, 94, 118, 51, 87, 130, 123, 180, 100, 136, 196, 57, 178,
                94, 78, 166, 183, 80, 5,
            ]).unwrap()),
        });

    }

    #[test]
    fn daemon_with_config_pubkey() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "http://127.0.0.1:7070"
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="
        "#).unwrap();
        let config = DaemonConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
            proxy: None,
            pubkey: Some(PublicKey::from_slice(&[
                115, 27, 214, 39, 98, 102, 27, 232, 92, 84, 12, 139, 20, 146,
                44, 161, 243, 112, 15, 176, 44, 198, 42, 22, 195, 238, 225,
                196, 48, 91, 122, 59,
            ]).unwrap()),
            seckey: None,
        });
    }

    #[test]
    fn daemon_with_arg_pubkey() {
        let args = Args {
            pubkey: Some("7MeJ1aZnUDzxBoZvMyx4UYS2M1KoR3j60kLWfzmMWAU=".to_string()),
            ..Default::default()
        };
        let mut config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "http://127.0.0.1:7070"
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="
        "#).unwrap();
        config.update(&args);

        let config = DaemonConfig::build(config, &args).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
            proxy: None,
            pubkey: Some(PublicKey::from_slice(&[
                236, 199, 137, 213, 166, 103, 80, 60, 241, 6, 134, 111, 51, 44,
                120, 81, 132, 182, 51, 82, 168, 71, 120, 250, 210, 66, 214,
                127, 57, 140, 88, 5,
            ]).unwrap()),
            seckey: None,
        });
    }

    #[test]
    fn daemon_with_pubkey_from_alias() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "@home"
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="

[destinations.home]
destination = "http://127.0.0.1:7070"
pubkey = "8S3Esx/GSsWZZbfcp4XO/stoyA/ABCE9xXaqM53kEgM="
        "#).unwrap();
        let config = DaemonConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
            proxy: None,
            pubkey: Some(PublicKey::from_slice(&[
                241, 45, 196, 179, 31, 198, 74, 197, 153, 101, 183, 220, 167,
                133, 206, 254, 203, 104, 200, 15, 192, 4, 33, 61, 197, 118,
                170, 51, 157, 228, 18, 3,
            ]).unwrap()),
            seckey: None,
        });
    }

    #[test]
    fn daemon_with_pubkey_from_arg_despite_alias() {
        let args = Args {
            pubkey: Some("7MeJ1aZnUDzxBoZvMyx4UYS2M1KoR3j60kLWfzmMWAU=".to_string()),
            ..Default::default()
        };
        let mut config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "@home"
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="

[destinations.home]
destination = "http://127.0.0.1:7070"
pubkey = "8S3Esx/GSsWZZbfcp4XO/stoyA/ABCE9xXaqM53kEgM="
        "#).unwrap();
        config.update(&args);

        let config = DaemonConfig::build(config, &args).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
            proxy: None,
            pubkey: Some(PublicKey::from_slice(&[
                236, 199, 137, 213, 166, 103, 80, 60, 241, 6, 134, 111, 51, 44,
                120, 81, 132, 182, 51, 82, 168, 71, 120, 250, 210, 66, 214,
                127, 57, 140, 88, 5,
            ]).unwrap()),
            seckey: None,
        });
    }

    #[test]
    fn daemon_with_pubkey_alias_in_dest() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "http://127.0.0.1:7070"
pubkey = "@home"

[pubkeys.home]
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="
        "#).unwrap();
        let config = DaemonConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
            proxy: None,
            pubkey: Some(PublicKey::from_slice(&[
                115, 27, 214, 39, 98, 102, 27, 232, 92, 84, 12, 139, 20, 146,
                44, 161, 243, 112, 15, 176, 44, 198, 42, 22, 195, 238, 225,
                196, 48, 91, 122, 59,
            ]).unwrap()),
            seckey: None,
        });
    }

    #[test]
    fn daemon_with_pubkey_alias_in_destination_alias() {
        let config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "@home"
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="

[destinations.home]
destination = "http://127.0.0.1:7070"
pubkey = "@foo"

[pubkeys.foo]
pubkey = "8S3Esx/GSsWZZbfcp4XO/stoyA/ABCE9xXaqM53kEgM="
        "#).unwrap();
        let config = DaemonConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
            proxy: None,
            pubkey: Some(PublicKey::from_slice(&[
                241, 45, 196, 179, 31, 198, 74, 197, 153, 101, 183, 220, 167,
                133, 206, 254, 203, 104, 200, 15, 192, 4, 33, 61, 197, 118,
                170, 51, 157, 228, 18, 3,
            ]).unwrap()),
            seckey: None,
        });
    }

    #[test]
    fn daemon_with_pubkey_from_arg_despite_key_in_dest_alias() {
        let args = Args {
            pubkey: Some("7MeJ1aZnUDzxBoZvMyx4UYS2M1KoR3j60kLWfzmMWAU=".to_string()),
            ..Default::default()
        };
        let mut config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "@home"
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="

[destinations.home]
destination = "http://127.0.0.1:7070"
pubkey = "@foo"

[pubkeys.foo]
pubkey = "8S3Esx/GSsWZZbfcp4XO/stoyA/ABCE9xXaqM53kEgM="
        "#).unwrap();
        config.update(&args);

        let config = DaemonConfig::build(config, &args).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
            proxy: None,
            pubkey: Some(PublicKey::from_slice(&[
                236, 199, 137, 213, 166, 103, 80, 60, 241, 6, 134, 111, 51, 44,
                120, 81, 132, 182, 51, 82, 168, 71, 120, 250, 210, 66, 214,
                127, 57, 140, 88, 5,
            ]).unwrap()),
            seckey: None,
        });
    }

    #[test]
    fn daemon_with_pubkey_from_arg_alias_despite_key_in_dest_alias() {
        let args = Args {
            pubkey: Some("@bar".to_string()),
            ..Default::default()
        };
        let mut config = ConfigFile::load_slice(br#"
[daemon]
socket = "/asdf/brchd.socket"
destination = "@home"
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="

[destinations.home]
destination = "http://127.0.0.1:7070"
pubkey = "@foo"

[pubkeys.foo]
pubkey = "7MeJ1aZnUDzxBoZvMyx4UYS2M1KoR3j60kLWfzmMWAU="

[pubkeys.bar]
pubkey = "8S3Esx/GSsWZZbfcp4XO/stoyA/ABCE9xXaqM53kEgM="
        "#).unwrap();
        config.update(&args);

        let config = DaemonConfig::build(config, &args).unwrap();
        assert_eq!(config, DaemonConfig {
            socket: PathBuf::from("/asdf/brchd.socket"),
            destination: "http://127.0.0.1:7070".to_string(),
            concurrency: 3,
            proxy: None,
            pubkey: Some(PublicKey::from_slice(&[
                241, 45, 196, 179, 31, 198, 74, 197, 153, 101, 183, 220, 167,
                133, 206, 254, 203, 104, 200, 15, 192, 4, 33, 61, 197, 118,
                170, 51, 157, 228, 18, 3,
            ]).unwrap()),
            seckey: None,
        });
    }
}
