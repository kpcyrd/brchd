use crate::errors::*;
use std::net::SocketAddr;
use structopt::StructOpt;
use structopt::clap::{AppSettings, Shell};

#[derive(Debug, StructOpt)]
#[structopt(global_settings = &[AppSettings::ColoredHelp])]
pub struct Args {
    pub paths: Vec<String>,
    #[structopt(short="d", long, group="action")]
    pub daemon: bool,
    #[structopt(short="H", long, group="action")]
    pub http_daemon: bool,
    #[structopt(long, possible_values=&Shell::variants(), group="action")]
    pub gen_completions: Option<Shell>,
    #[structopt(short="p", long)]
    pub upload_dest: Option<String>,
    #[structopt(short="B", long, parse(try_from_str = parse_addr))]
    pub bind_addr: Option<SocketAddr>,
    #[structopt(short="w", long, group="action")]
    pub wait: bool,
    // TODO: ~/.local/share/brchd.sock
    // TODO: if not set, read from environment variable
    #[structopt(short="S", long, default_value="brchd.sock")]
    pub socket: String,
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
