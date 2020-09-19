pub mod args;
pub mod config;

#[cfg(feature = "crypto")]
#[path="crypto/mod.rs"]
pub mod crypto;
#[cfg(not(feature = "crypto"))]
#[path="crypto/shim.rs"]
pub mod crypto;

pub mod daemon;
pub mod destination;
pub mod errors;
#[cfg(feature = "spider")]
pub mod html;

#[cfg(feature = "httpd")]
#[path="httpd/mod.rs"]
pub mod httpd;
#[cfg(not(feature = "httpd"))]
#[path="httpd/shim.rs"]
pub mod httpd;

pub mod ipc;
pub mod pathspec;
pub mod queue;

#[cfg(feature = "spider")]
#[path="spider/mod.rs"]
pub mod spider;
#[cfg(not(feature = "spider"))]
#[path="spider/shim.rs"]
pub mod spider;

pub mod standalone;
pub mod status;
pub mod temp;
pub mod uploader;
pub mod walkdir;
pub mod web;
