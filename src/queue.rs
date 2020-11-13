use crate::args::Args;
use crate::config::{ClientConfig, DaemonConfig};
use crate::errors::*;
use crate::ipc::IpcClient;
use crate::spider;
use crate::standalone::Standalone;
use crate::walkdir;
use crate::web;
use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub target: Target,
    pub size: u64,
}

impl Task {
    pub fn path(path: PathBuf, resolved: PathBuf, size: u64) -> Task {
        Task {
            target: Target::Path(PathTarget {
                path,
                resolved,
            }),
            size,
        }
    }

    pub fn url(path: String, url: Url) -> Task {
        Task {
            target: Target::Url(UrlTarget {
                path,
                url,
            }),
            size: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Target {
    Path(PathTarget),
    Url(UrlTarget),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathTarget {
    pub path: PathBuf,
    pub resolved: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlTarget {
    pub path: String,
    pub url: Url,
}

pub trait QueueClient {
    fn push_work(&mut self, task: Task) -> Result<()>;

    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}

pub fn run_add(args: Args) -> Result<()> {
    let config = ClientConfig::load(&args)?;

    let client: Box<dyn QueueClient> = if args.destination.is_some() {
        let config = DaemonConfig::load(&args)?;
        Box::new(Standalone::new(&args, config)?)
    } else {
        Box::new(IpcClient::connect(config.socket)?)
    };

    let http = web::client(Some(Duration::from_secs(60)), config.proxy.as_ref(), args.accept_invalid_certs, args.user_agent.as_ref())?;
    exec(args, client, http)
}

pub fn exec(args: Args, mut client: Box<dyn QueueClient>, http: Client) -> Result<()> {
    for path in &args.paths {
        if path.starts_with("https://") || path.starts_with("https://") {
            spider::queue(client.as_mut(), &http, path)?;
        } else {
            walkdir::queue(client.as_mut(), path)?;
        }
    }
    client.finish()
}
