use crate::errors::*;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use structopt::StructOpt;
use structopt::clap::{AppSettings, Shell};

#[derive(Debug, StructOpt)]
#[structopt(global_settings = &[AppSettings::ColoredHelp])]
pub struct Args {
    pub paths: Vec<String>,
    /// Run the uploader daemon
    #[structopt(short="d", long, group="action")]
    pub daemon: bool,
    /// Run the http uploads receiver
    #[structopt(short="H", long, group="action")]
    pub http_daemon: bool,
    /// Generate shell completions
    #[structopt(long, possible_values=&Shell::variants(), group="action")]
    pub gen_completions: Option<Shell>,
    /// Directory to store uploads in
    #[structopt(short="p", long)]
    pub upload_dest: Option<String>,
    /// Address to bind to
    #[structopt(short="B", long, parse(try_from_str = parse_addr))]
    pub bind_addr: Option<SocketAddr>,
    /// Concurrent uploads
    #[structopt(short="n", default_value="3")]
    pub concurrency: usize,
    /// Block until all pending uploads are done
    #[structopt(short="w", long, group="action")]
    pub wait: bool,
    #[structopt(short="S", long, env="BRCHD_SOCK")]
    pub socket: Option<PathBuf>,
}

impl Args {
    pub fn socket(&self) -> Result<PathBuf> {
        if let Some(path) = &self.socket {
            Ok(path.clone())
        } else {
            let path = dirs::data_dir()
                .ok_or_else(|| format_err!("Failed to find data directory"))?;

            fs::create_dir_all(&path)
                .context("Failed to create data directory")?;

            Ok(path.join("brchd.sock"))
        }
    }
}

fn parse_addr(s: &str) -> Result<SocketAddr> {
    let idx = s
        .find(':')
        .ok_or_else(|| format_err!("no `:` found in `{}`", s))?;

    let r = if idx == 0 {
        let s = format!("[::]{}", s);
        s.parse::<SocketAddr>()
    } else {
        s.parse::<SocketAddr>()
    }?;
    Ok(r)
}
