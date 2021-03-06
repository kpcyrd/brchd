use crate::args::Args;
use crate::config::{self, ConfigFile};
use crate::crypto::{self, PublicKey, SecretKey};
use crate::errors::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct EncryptConfig {
    pub pubkey: PublicKey,
    pub seckey: Option<SecretKey>,
}

impl EncryptConfig {
    pub fn load(args: &Args) -> Result<EncryptConfig> {
        let config = ConfigFile::load(args)?;
        Self::build(config, args)
    }

    fn build(config: ConfigFile, _args: &Args) -> Result<EncryptConfig> {
        let pubkey = config.crypto.pubkey
            .ok_or_else(|| format_err!("public key is missing"))?;

        let pubkey = if let Some(alias) = config::resolve_alias(&config.pubkeys, &pubkey)? {
            &alias.pubkey
        } else {
            &pubkey
        };
        let pubkey = crypto::decode_pubkey(&pubkey)?;

        let seckey = if let Some(seckey) = config.crypto.seckey {
            let seckey = crypto::decode_seckey(&seckey)?;
            Some(seckey)
        } else {
            None
        };

        Ok(EncryptConfig {
            pubkey,
            seckey,
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DecryptConfig {
    pub pubkey: Option<PublicKey>,
    pub seckey: SecretKey,
}

impl DecryptConfig {
    pub fn load(args: &Args) -> Result<DecryptConfig> {
        let config = ConfigFile::load(args)?;
        Self::build(config, args)
    }

    fn build(config: ConfigFile, _args: &Args) -> Result<DecryptConfig> {
        let pubkey = if let Some(pubkey) = config.crypto.pubkey {
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

        let seckey = config.crypto.seckey
            .ok_or_else(|| format_err!("secret key is missing"))?;
        let seckey = crypto::decode_seckey(&seckey)?;

        Ok(DecryptConfig {
            pubkey,
            seckey,
        })
    }
}


#[cfg(all(test, feature="crypto"))]
mod tests {
    use super::*;

    #[test]
    fn default_encrypt_config() {
        let config = ConfigFile::load_slice(br#"
[crypto]
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="
        "#).unwrap();
        let config = EncryptConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, EncryptConfig {
            pubkey: PublicKey::from_slice(&[
                115, 27, 214, 39, 98, 102, 27, 232, 92, 84, 12, 139, 20, 146,
                44, 161, 243, 112, 15, 176, 44, 198, 42, 22, 195, 238, 225,
                196, 48, 91, 122, 59,
            ]).unwrap(),
            seckey: None,
        });
    }

    #[test]
    fn encrypt_pubkey_arg() {
        let args = Args {
            pubkey: Some("cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs=".to_string()),
            ..Default::default()
        };
        let mut config = ConfigFile::load_slice(br#""#).unwrap();
        config.update(&args);
        let config = EncryptConfig::build(config, &args).unwrap();
        assert_eq!(config, EncryptConfig {
            pubkey: PublicKey::from_slice(&[
                115, 27, 214, 39, 98, 102, 27, 232, 92, 84, 12, 139, 20, 146,
                44, 161, 243, 112, 15, 176, 44, 198, 42, 22, 195, 238, 225,
                196, 48, 91, 122, 59,
            ]).unwrap(),
            seckey: None,
        });
    }

    #[test]
    fn default_decrypt_config() {
        let config = ConfigFile::load_slice(br#"
[crypto]
seckey = "5LYdSbVM3Pxnvzi71bZedjNXgnu0ZIjEObJeTqa3UAU="
        "#).unwrap();
        let config = DecryptConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DecryptConfig {
            seckey: SecretKey::from_slice(&[
                228, 182, 29, 73, 181, 76, 220, 252, 103, 191, 56, 187, 213,
                182, 94, 118, 51, 87, 130, 123, 180, 100, 136, 196, 57, 178,
                94, 78, 166, 183, 80, 5,
            ]).unwrap(),
            pubkey: None,
        });
    }

    #[test]
    fn encrypt_resolve_alias() {
        let config = ConfigFile::load_slice(br#"
[pubkeys.home]
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="

[crypto]
pubkey = "@home"
        "#).unwrap();
        let config = EncryptConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, EncryptConfig {
            pubkey: PublicKey::from_slice(&[
                115, 27, 214, 39, 98, 102, 27, 232, 92, 84, 12, 139, 20, 146,
                44, 161, 243, 112, 15, 176, 44, 198, 42, 22, 195, 238, 225,
                196, 48, 91, 122, 59,
            ]).unwrap(),
            seckey: None,
        });
    }

    #[test]
    fn encrypt_resolve_alias_arg() {
        let args = Args {
            pubkey: Some("@home".to_string()),
            ..Default::default()
        };
        let mut config = ConfigFile::load_slice(br#"
[pubkeys.home]
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="
        "#).unwrap();
        config.update(&args);
        let config = EncryptConfig::build(config, &args).unwrap();
        assert_eq!(config, EncryptConfig {
            pubkey: PublicKey::from_slice(&[
                115, 27, 214, 39, 98, 102, 27, 232, 92, 84, 12, 139, 20, 146,
                44, 161, 243, 112, 15, 176, 44, 198, 42, 22, 195, 238, 225,
                196, 48, 91, 122, 59,
            ]).unwrap(),
            seckey: None,
        });
    }

    #[test]
    fn encrypt_invalid_alias() {
        let config = ConfigFile::load_slice(br#"
[crypto]
pubkey = "@home"
        "#).unwrap();
        let r = EncryptConfig::build(config, &Args::default());
        assert!(r.is_err());
    }

    #[test]
    fn decrypt_with_strict_key_alias() {
        let config = ConfigFile::load_slice(br#"
[crypto]
pubkey = "@home"
seckey = "4pYQbPj4mpIFGasDsa73tqrvKtOi51uL6vfXUBQzR7w="

[pubkeys.home]
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="
        "#).unwrap();
        let config = DecryptConfig::build(config, &Args::default()).unwrap();
        assert_eq!(config, DecryptConfig {
            seckey: SecretKey::from_slice(&[
                226, 150, 16, 108, 248, 248, 154, 146, 5, 25, 171, 3, 177, 174,
                247, 182, 170, 239, 42, 211, 162, 231, 91, 139, 234, 247, 215,
                80, 20, 51, 71, 188,
            ]).unwrap(),
            pubkey: Some(PublicKey::from_slice(&[
                115, 27, 214, 39, 98, 102, 27, 232, 92, 84, 12, 139, 20, 146,
                44, 161, 243, 112, 15, 176, 44, 198, 42, 22, 195, 238, 225,
                196, 48, 91, 122, 59,
            ]).unwrap()),
        });
    }
}
