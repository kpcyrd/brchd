use crate::args::Args;
use crate::config::ConfigFile;
use crate::errors::*;
use serde::{Serialize, Deserialize};
use sodiumoxide::crypto::box_::{PublicKey, SecretKey};
// use std::collections::HashMap;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct EncryptConfig {
    pub pubkey: PublicKey,
}

impl EncryptConfig {
    pub fn load(args: &Args) -> Result<EncryptConfig> {
        let config = ConfigFile::load(args)?;
        Self::build(config)
    }

    fn build(config: ConfigFile) -> Result<EncryptConfig> {
        let pubkey = config.crypto.pubkey
            .ok_or_else(|| format_err!("public key is missing"))?;
        let pubkey = base64::decode(&pubkey)
            .context("Failed to base64 decode public key")?;
        let pubkey = PublicKey::from_slice(&pubkey)
            .ok_or_else(|| format_err!("Wrong length for public key"))?;

        Ok(EncryptConfig {
            pubkey,
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DecryptConfig {
    pub seckey: SecretKey,
}

impl DecryptConfig {
    pub fn load(args: &Args) -> Result<DecryptConfig> {
        let config = ConfigFile::load(args)?;
        Self::build(config)
    }

    fn build(config: ConfigFile) -> Result<DecryptConfig> {
        let seckey = config.crypto.seckey
            .ok_or_else(|| format_err!("secret key is missing"))?;
        let seckey = base64::decode(&seckey)
            .context("Failed to base64 decode secret key")?;
        let seckey = SecretKey::from_slice(&seckey)
            .ok_or_else(|| format_err!("Wrong length for secret key"))?;

        Ok(DecryptConfig {
            seckey,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_encrypt_config() {
        let config = ConfigFile::load_slice(br#"
[crypto]
pubkey = "cxvWJ2JmG+hcVAyLFJIsofNwD7AsxioWw+7hxDBbejs="
        "#).unwrap();
        let config = EncryptConfig::build(config).unwrap();
        assert_eq!(config, EncryptConfig {
            pubkey: PublicKey::from_slice(&[
                115, 27, 214, 39, 98, 102, 27, 232, 92, 84, 12, 139, 20, 146,
                44, 161, 243, 112, 15, 176, 44, 198, 42, 22, 195, 238, 225,
                196, 48, 91, 122, 59,
            ]).unwrap(),
        });
    }

    #[test]
    fn default_decrypt_config() {
        let config = ConfigFile::load_slice(br#"
[crypto]
seckey = "5LYdSbVM3Pxnvzi71bZedjNXgnu0ZIjEObJeTqa3UAU="
        "#).unwrap();
        let config = DecryptConfig::build(config).unwrap();
        assert_eq!(config, DecryptConfig {
            seckey: SecretKey::from_slice(&[
                228, 182, 29, 73, 181, 76, 220, 252, 103, 191, 56, 187, 213,
                182, 94, 118, 51, 87, 130, 123, 180, 100, 136, 196, 57, 178,
                94, 78, 166, 183, 80, 5,
            ]).unwrap(),
        });
    }
}
