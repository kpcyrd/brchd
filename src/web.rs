use crate::errors::*;
use std::convert::TryFrom;
use std::time::Duration;
use reqwest::{blocking::Client, header::HeaderValue, Proxy};

pub fn client(timeout: Option<Duration>, proxy: Option<&String>, accept_invalid_certs: bool, user_agent: Option<&String>) -> Result<Client> {
    let mut builder = Client::builder()
        .user_agent(if let Some(user_agent) = user_agent {
            HeaderValue::try_from(user_agent)?
        } else {
            let user_agent = format!("brchd/{}", env!("CARGO_PKG_VERSION"));
            HeaderValue::try_from(user_agent)?
        })
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
