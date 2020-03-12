use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub enum Item {
    Path(PathBuf),
    Url(Url),
}
