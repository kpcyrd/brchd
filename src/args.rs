use crate::errors::*;
use std::net::{SocketAddr, IpAddr};
use std::path::PathBuf;
use structopt::StructOpt;
use structopt::clap::{AppSettings, Shell};

#[derive(Debug, Default, StructOpt)]
#[structopt(global_settings = &[AppSettings::ColoredHelp])]
pub struct Args {
    /// Verbose output
    #[structopt(short="v", parse(from_occurrences))]
    pub verbose: u8,
    /// Perform the action on the given paths
    pub paths: Vec<String>,
    /// Run the uploader daemon
    #[structopt(short="D", long, group="action")]
    pub daemon: bool,
    /// Run the http uploads receiver
    #[structopt(short="H", long, group="action")]
    pub http_daemon: bool,
    /// Show upload queue
    #[structopt(short="Q", long, group="action")]
    pub queue: bool,

    /// Encrypt files
    #[structopt(long, group="action")]
    pub encrypt: bool,
    /// Decrypt files
    #[structopt(long, group="action")]
    pub decrypt: bool,
    /// Generate a keypair for encryption
    #[structopt(long, group="action")]
    pub keygen: bool,

    /// Generate shell completions
    #[structopt(long, possible_values=&Shell::variants(), group="action")]
    pub gen_completions: Option<Shell>,
    /// Storage destination
    #[structopt(short="d", long)]
    pub destination: Option<String>,
    /// Address to bind to
    #[structopt(short="B", long, parse(try_from_str = parse_addr))]
    pub bind_addr: Option<SocketAddr>,
    /// Concurrent uploads
    #[structopt(short="n")]
    pub concurrency: Option<usize>,
    /// Block until all pending uploads are done
    #[structopt(short="w", long, group="action")]
    pub wait: bool,
    /// Use the given path for the brchd socket
    #[structopt(short="S", long, env="BRCHD_SOCK")]
    pub socket: Option<PathBuf>,
    /// Load the configuration at the given path
    #[structopt(short="c", long, env="BRCHD_CONFIG")]
    pub config: Option<PathBuf>,
    /// Store uploads with the given pattern
    #[structopt(short="F", long, env="BRCHD_PATH_FORMAT")]
    pub path_format: Option<String>,
    /// Encrypt for the given public key
    #[structopt(long, env="BRCHD_PUBKEY")]
    pub pubkey: Option<String>,
    /// Decrypt with the given secret key
    #[structopt(long, env="BRCHD_SECKEY")]
    pub seckey: Option<String>,
    /// Use a proxy for all requests (e.g. socks5://127.0.0.1:9050)
    #[structopt(short="x", long)]
    pub proxy: Option<String>,
    /// Accept invalid certificates. The certificate may be expired, not valid for this hostname or lack signatures from a trusted CA. This makes the connection vulnerable to mitm attacks.
    #[structopt(long)]
    pub accept_invalid_certs: bool,
}

fn parse_addr(s: &str) -> Result<SocketAddr> {
    let idx = s
        .find(':')
        .ok_or_else(|| format_err!("no `:` found in `{}`", s))?;

    let r = if idx == 0 {
        let port = s[1..].parse()?;
        SocketAddr::new(IpAddr::from([
            0, 0, 0, 0,
            0, 0, 0, 0,
        ]), port)
    } else {
        s.parse::<SocketAddr>()?
    };
    Ok(r)
}
