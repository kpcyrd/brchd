use crate::errors::*;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub target: Target,
    pub size: u64,
}

impl Task {
    pub fn path(path: PathBuf, size: u64) -> Task {
        Task {
            target: Target::Path(path),
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
    Path(PathBuf),
    Url(Url),
}

pub trait QueueClient {
    fn push_work(&mut self, task: Task) -> Result<()>;
}
