use crate::errors::*;
use crate::html;
use crate::ipc::IpcClient;
use crate::queue::Item;
use reqwest::Client;
use std::collections::VecDeque;
use url::Url;

pub async fn queue(client: &mut IpcClient, http: &Client, target: &str) -> Result<()> {
    let mut queue = VecDeque::new();

    let target = target.parse::<Url>()
        .context("Failed to parse target as url")?;
    queue.push_back(target);

    while let Some(target) = queue.pop_front() {
        let resp = http.get(target.clone())
            .send()
            .await?
            .error_for_status()?;

        let body = resp.text().await?;
        let links = html::parse_links(body.as_bytes())?;

        for link in &links {
            let link = target.join(link)?;
            let link_str = link.as_str();
            let target = target.as_str();

            if !link_str.starts_with(target) || link_str == target {
                debug!("Not a child link, skipping");
                continue;
            }

            if link_str.ends_with('/') {
                info!("traversing into directory: {:?}", link_str);
                queue.push_back(link);
            } else {
                let item = Item::url(link);
                client.push_work(item)?;
            }
        }
    }

    Ok(())
}
