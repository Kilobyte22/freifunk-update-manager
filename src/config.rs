use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub listen: SocketAddr,
    pub sites: Vec<SiteConfig>
}

#[derive(Deserialize, Debug, Clone)]
pub struct SiteConfig {
    pub enabled: bool,
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
    pub refresh_interval: u64,
    #[serde(rename = "update-timeout")]
    pub update_timeout: u64,
    #[serde(rename = "broken-threshold")]
    pub broken_threshold: u64,
    #[serde(rename = "state-file")]
    pub state_file: PathBuf
}