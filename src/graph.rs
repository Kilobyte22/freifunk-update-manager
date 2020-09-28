use crate::meshinfo::MeshInfo;
use slotmap::{DenseSlotMap, SecondaryMap};
use std::collections::HashMap;
use std::net::IpAddr;
use std::cmp;
use crate::config::SiteConfig;
slotmap::new_key_type! { pub struct NodeKey; }

pub struct Graph {
    pub nodes: DenseSlotMap<NodeKey, NodeContainer>,
    pub ip_addrs: HashMap<IpAddr, NodeKey>,
    pub depths: SecondaryMap<NodeKey, u8>,
    pub max_depth: u8,
    pub update_policy: SecondaryMap<NodeKey, UpdatePolicy>,
}

impl Graph {
    pub fn build(info: &MeshInfo, config: &SiteConfig) -> Graph {
        let mut nodes = DenseSlotMap::with_capacity_and_key(info.nodes.len());
        let mut id_lookup = HashMap::<crate::meshinfo::NodeID, NodeKey>::new();
        let mut ip_addrs = HashMap::new();

        log::debug!("Graph building pass 1: Setting up data");
        for node in &info.nodes {
            let key = nodes.insert(NodeContainer {
                node: (*node).clone(),
                uplink: None,
                downlinks: vec![]
            });

            id_lookup.insert(node.node_id.clone(), key);
            for addr in &node.addresses {
                ip_addrs.insert(addr.clone(), key);
            }
        }

        log::debug!("Graph building pass 2: building links");
        for (_, node) in &mut nodes {
            if let Some(uplink) = &node.node.gateway_nexthop {
                node.uplink = id_lookup.get(uplink).map(|key| *key)
            }
        }

        log::debug!("Graph building pass 2: calculating node depth");
        let mut depths = SecondaryMap::with_capacity(nodes.len());

        let mut max_depth = 0;

        {
            let mut node_list: Vec<_> = nodes.keys().collect();

            let mut i = 0;

            while !node_list.is_empty() {
                i = i % node_list.len();
                let key = *node_list.get(i).unwrap();
                let mut remove = false;
                {
                    let node = nodes.get(key).unwrap();
                    let mut set_depth = None;
                    if let Some(uplink_key) = node.uplink {
                        if let Some(uplink_depth) = depths.get(uplink_key) {
                            set_depth = Some(uplink_depth + 1);
                        }
                    } else {
                        set_depth = Some(0);
                    }

                    if let Some(d) = set_depth {
                        depths.insert(key, d);
                        remove = true;
                        max_depth = cmp::max(d, max_depth)
                    }
                }
                if remove {
                    node_list.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        let mut update_policy = SecondaryMap::new();

        for (key, node) in &nodes {
            let mut policy = UpdatePolicy::Ready;
            if node.node.firmware.release == config.latest_version {
                policy = UpdatePolicy::Finished;
            } else {
                for downlink_key in &node.downlinks {
                    let downlink = nodes.get(*downlink_key).unwrap();
                    if downlink.node.firmware.release != config.latest_version {
                        if downlink.node.autoupdater.enabled || !config.ignore_autoupdate_off {
                            policy = UpdatePolicy::Pending;
                        }
                    }
                }
            }

            update_policy.insert(key, policy);
        }

        Graph {
            nodes,
            ip_addrs,
            depths,
            max_depth,
            update_policy
        }
    }
}

pub struct NodeContainer {
    pub node: crate::meshinfo::Node,
    pub uplink: Option<NodeKey>,
    pub downlinks: Vec<NodeKey>
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum UpdatePolicy {
    Pending,
    Ready,
    Finished
}