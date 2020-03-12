use crate::errors::*;
use crate::ipc::IpcClient;
use crate::queue::Item;
use walkdir::WalkDir;

pub fn queue(client: &mut IpcClient, target: &str) -> Result<()> {
    for entry in WalkDir::new(target) {
        let entry = entry?;

        if entry.file_type().is_file() {
            let item = Item::Path(entry.into_path());
            client.push_work(item)?;
        }
    }

    Ok(())
}
