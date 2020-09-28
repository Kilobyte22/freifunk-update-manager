use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub listen: SocketAddr,
    pub sites: Vec<SiteConfig>
}

#[derive(Deserialize, Debug, Clone)]
pub struct SiteConfig {
    #[serde(rename = "latest-version")]
    pub latest_version: String,
    pub name: String,
    pub branch: String,
    pub meshinfo: String,
    #[serde(rename = "on-update")]
    pub on_update: String,
    #[serde(rename = "on-noupdate")]
    pub on_noupdate: String,
    #[serde(rename = "update-default")]
    pub update_default: bool,
    #[serde(rename = "dry-run")]
    pub dry_run: bool,
    #[serde(rename = "ignore-autoupdate-off")]
    pub ignore_autoupdate_off: bool,
    #[serde(rename = "refresh-interval")]
    pub refresh_interval: u64
}