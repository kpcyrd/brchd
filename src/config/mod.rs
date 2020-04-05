use crate::errors::*;
use std::collections::HashMap;
use std::net::{SocketAddr, IpAddr, Ipv6Addr};

const DEFAULT_CONCURRENCY: usize = 3;
const DEFAULT_PATH_FORMAT: &str = "%p";

mod file;
pub use self::file::ConfigFile;
mod daemon;
pub use self::daemon::DaemonConfig;
mod client;
pub use self::client::ClientConfig;
mod upload;
pub use self::upload::UploadConfig;
mod crypto;
pub use self::crypto::{EncryptConfig, DecryptConfig};

#[inline(always)]
fn default_port() -> SocketAddr {
    SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)),
        7070,
    )
}

fn resolve_alias<'a, T>(aliases: &'a HashMap<String, T>, alias: &str) -> Result<Option<&'a T>> {
    if alias.starts_with('@') {
        let value = aliases.get(&alias[1..])
            .ok_or_else(|| format_err!("Failed to resolve alias"))?;
        Ok(Some(value))
    } else {
        Ok(None)
    }
}
