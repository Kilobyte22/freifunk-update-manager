use serde::Deserialize;
use std::net::IpAddr;
use std::fmt;
use crate::mac::MacAddr;

#[derive(Deserialize, Debug)]
pub struct MeshInfo {
    pub timestamp: chrono::DateTime<chrono::offset::Utc>,
    pub nodes: Vec<Node>,
    pub links: Vec<Link>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Node {
    #[serde(rename = "firstseen")]
    pub first_seen: chrono::DateTime<chrono::offset::Utc>,
    #[serde(rename = "lastseen")]
    pub last_seen: chrono::DateTime<chrono::offset::Utc>,
    pub is_online: bool,
    pub is_gateway: bool,
    pub clients: u32,
    pub clients_wifi24: u32,
    #[serde(default)]
    pub client_wifi5: u32,
    pub clients_other: u32,
    pub rootfs_usage: f32,
    pub loadavg: f32,
    pub memory_usage: f32,
    pub uptime: chrono::DateTime<chrono::offset::Utc>,
    pub gateway_nexthop: Option<NodeID>,
    pub gateway: Option<NodeID>,
    pub node_id: NodeID,
    pub mac: MacAddr,
    pub addresses: Vec<IpAddr>,
    pub domain: String,
    pub hostname: String,
    pub owner: Option<String>,
    pub location: Option<Location>,
    pub firmware: FirmwareInfo,
    pub autoupdater: Autoupdater,
    pub nproc: u16,
    pub model: Option<String>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Location {
    pub longitude: f64,
    pub latitude: f64
}

#[derive(Deserialize, Debug, Clone)]
pub struct FirmwareInfo {
    pub base: String,
    pub release: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Autoupdater {
    pub enabled: bool,
    pub branch: Option<String>
}

#[derive(Deserialize, Debug, Clone)]
pub struct Link {
    #[serde(rename = "type")]
    pub ty: LinkType,
    pub source: NodeID,
    pub target: NodeID,
    pub source_tq: f32,
    pub target_tq: f32,
    pub source_addr: MacAddr,
    pub target_addr: MacAddr,
}

#[derive(Deserialize, Debug, Clone)]
pub enum LinkType {
    #[serde(rename = "wifi")]
    Wireless,
    #[serde(rename = "vpn")]
    VPN,
    #[serde(rename = "other")]
    Other
}

#[derive(Deserialize, Debug, Eq, PartialEq, Hash, Clone)]
#[serde(transparent)]
pub struct NodeID(String);

impl fmt::Display for NodeID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}