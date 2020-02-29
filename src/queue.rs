use std::path::PathBuf;
use url::Url;

#[derive(Debug)]
pub enum Item {
    Path(PathBuf),
    Url(Url),
}
