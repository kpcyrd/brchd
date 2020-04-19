use crate::errors::*;
use crate::queue:: {QueueClient};
use reqwest::blocking::Client;

pub fn queue(_: &mut dyn QueueClient, _: &Client, _: &str) -> Result<()> {
    unimplemented!()
}
