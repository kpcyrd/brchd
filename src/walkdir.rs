use crate::errors::*;
use crate::queue::Item;
use reqwest::Client;
use walkdir::WalkDir;

pub fn queue(_client: &Client, target: &str) -> Result<()> {
    for entry in WalkDir::new(target) {
        let entry = entry?;

        if entry.file_type().is_file() {
            let item = Item::Path(entry.into_path());
            println!("queue item: {:?}", item);
        }
    }

    Ok(())
}
