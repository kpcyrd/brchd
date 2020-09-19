use crate::errors::*;
use std::time::Duration;
use reqwest::{blocking::Client, Proxy};

pub fn client(timeout: Option<Duration>, proxy: Option<&String>, accept_invalid_certs: bool) -> Result<Client> {
    let mut builder = Client::builder()
        .danger_accept_invalid_certs(accept_invalid_certs)
        .connect_timeout(Duration::from_secs(5));

    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout);
    }
    if let Some(proxy) = proxy {
        builder = builder.proxy(Proxy::all(proxy)?);
    }
    let http = builder.build()?;
    Ok(http)
}
