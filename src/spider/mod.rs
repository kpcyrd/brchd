use crate::errors::*;
use crate::html;
use crate::queue:: {Task, QueueClient};
use reqwest::blocking::Client;
use std::collections::VecDeque;
use url::Url;

pub fn queue(client: &mut dyn QueueClient, http: &Client, base: &str) -> Result<()> {
    let mut queue = VecDeque::new();

    let target = base.parse::<Url>()
        .context("Failed to parse target as url")?;
    queue.push_back(target);

    while let Some(target) = queue.pop_front() {
        debug!("Fetching directory listing from url {:?}", target);
        let resp = http.get(target.clone())
            .send()
            .context("Failed to send request")?
            .error_for_status()
            .context("Got http error")?;

        let body = resp.text()?;
        debug!("Downloaded {} bytes", body.as_bytes().len());
        let links = html::parse_links(body.as_bytes())?;
        debug!("Discovered {} <a> tags", links.len());

        for href in &links {
            debug!("Discovered href: {:?}", href);
            let link = target.join(href)?;
            let link_str = link.as_str();
            debug!("Discovered link: {:?}", link_str);
            let target = target.as_str();

            if !link_str.starts_with(target) || link_str == target {
                debug!("Not a child link, skipping");
                continue;
            }

            if link_str.ends_with('/') {
                info!("traversing into directory: {:?}", link_str);
                queue.push_back(link);
            } else {
                let relative = link_str[base.len()..].to_string();
                let task = Task::url(relative, link);
                client.push_work(task)?;
            }
        }
    }

    Ok(())
}
