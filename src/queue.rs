use crate::args::Args;
use crate::config::ClientConfig;
use crate::errors::*;
use crate::ipc::IpcClient;
use crate::spider;
use crate::standalone::Standalone;
use crate::walkdir;
use reqwest::Client;
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

    pub fn url(url: Url) -> Task {
        Task {
            target: Target::Url(url),
            size: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Target {
    Path(PathTarget),
    Url(Url),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathTarget {
    pub path: PathBuf,
    pub resolved: PathBuf,
}

pub trait QueueClient {
    fn push_work(&mut self, task: Task) -> Result<()>;
}

#[actix_rt::main]
pub async fn run_add(args: Args) -> Result<()> {
    let config = ClientConfig::load(&args)?;

    let mut client: Box<dyn QueueClient> = if let Some(dest) = args.destination {
        Box::new(Standalone::new(dest))
    } else {
        Box::new(IpcClient::connect(&config.socket)?)
    };

    let http = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(60))
        .build()?;

    for path in &args.paths {
        if path.starts_with("https://") || path.starts_with("https://") {
            spider::queue(&mut client, &http, path).await?;
        } else {
            walkdir::queue(&mut client, path)?;
        }
    }

    Ok(())
}
