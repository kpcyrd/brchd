use crate::errors::*;
use crate::ipc::IpcClient;
use crate::queue::Item;
use std::fs;
use walkdir::WalkDir;

pub fn queue(client: &mut IpcClient, target: &str) -> Result<()> {
    for entry in WalkDir::new(target) {
        let entry = entry?;
        let md = entry.metadata()?;

        if md.is_file() {
            let path = fs::canonicalize(entry.into_path())?;
            let item = Item::path(path, md.len());
            client.push_work(item)?;
        }
    }

    Ok(())
}
