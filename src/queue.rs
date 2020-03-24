use crate::errors::*;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub target: Target,
    pub size: u64,
}

impl Item {
    pub fn path(path: PathBuf, size: u64) -> Item {
        Item {
            target: Target::Path(path),
            size,
        }
    }

    pub fn url(url: Url) -> Item {
        Item {
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
    fn push_work(&mut self, task: Item) -> Result<()>;
}
