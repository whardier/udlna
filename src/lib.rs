//! Minimal DLNA/UPnP media server — scan directories, index media, and serve over HTTP.

pub mod cli;
pub mod cms;
pub mod config;
pub mod http;
pub mod media;
pub mod ssdp;
