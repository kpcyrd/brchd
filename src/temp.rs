use crate::errors::*;
use std::path::{Path, PathBuf};

pub fn partial_path(path: &Path) -> Result<(&Path, PathBuf)> {
    let parent = path.parent()
        .ok_or_else(|| format_err!("Path has no parent"))?;
    let filename = path.file_name()
        .ok_or_else(|| format_err!("Path has no file name"))?
        .to_str()
        .ok_or_else(|| format_err!("Filename contains invalid bytes"))?;

    let temp_filename = format!(".{}.part", filename);
    let temp_path = parent.join(temp_filename);
    Ok((parent, temp_path))
}
