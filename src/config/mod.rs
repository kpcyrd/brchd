use crate::errors::*;
use std::net::{SocketAddr, IpAddr, Ipv6Addr};
use std::path::PathBuf;

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

fn build_socket_path(socket: Option<PathBuf>, search: bool) -> Result<PathBuf> {
    if let Some(path) = socket {
        Ok(path)
    } else {
        let path = dirs::data_dir()
            .ok_or_else(|| format_err!("Failed to find data directory"))?;
        let path = path.join("brchd.sock");
        if !search || path.exists() {
            return Ok(path);
        }

        let path = PathBuf::from("/var/run/brchd/sock");
        if path.exists() {
            return Ok(path);
        }

        bail!("Could not find brchd socket, is brchd -D running?")
    }
}

#[inline(always)]
fn default_port() -> SocketAddr {
    SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)),
        7070,
    )
}
