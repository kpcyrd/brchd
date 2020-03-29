use crate::errors::*;
use crate::queue::{Task, QueueClient};
use std::fs;
use walkdir::WalkDir;

pub fn queue(client: &mut Box<dyn QueueClient>, target: &str) -> Result<()> {
    for entry in WalkDir::new(target) {
        let entry = entry?;
        debug!("walkdir: {:?}", entry);
        let md = entry.metadata()?;
        let ft = md.file_type();

        let path = entry.into_path();
        let resolved = fs::canonicalize(&path)?;

        let task = if ft.is_file() {
            Task::path(path, resolved, md.len())
        } else if ft.is_symlink() {
            debug!("resolving symlink: {:?}", path);
            let md = fs::metadata(&path)?;
            Task::path(path, resolved, md.len())
        } else {
            continue;
        };

        client.push_work(task)?;
    }

    Ok(())
}
