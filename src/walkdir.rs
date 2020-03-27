use crate::errors::*;
use crate::queue::{Item, QueueClient};
use std::fs;
use walkdir::WalkDir;

pub fn queue(client: &mut Box<dyn QueueClient>, target: &str) -> Result<()> {
    for entry in WalkDir::new(target) {
        let entry = entry?;
        debug!("walkdir: {:?}", entry);
        let md = entry.metadata()?;
        let ft = md.file_type();
        let path = fs::canonicalize(entry.into_path())?;

        let item = if ft.is_file() {
            Item::path(path, md.len())
        } else if ft.is_symlink() {
            debug!("resolving symlink: {:?}", path);
            let md = fs::metadata(&path)?;
            Item::path(path, md.len())
        } else {
            continue;
        };

        info!("pushing item to daemon: {:?}", item);
        client.push_work(item)?;
    }

    Ok(())
}
