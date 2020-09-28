use crate::meshinfo::MeshInfo;
use slotmap::{DenseSlotMap, SecondaryMap};
use std::collections::HashMap;
use std::net::IpAddr;
use std::{cmp, mem};
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

            let mut inner_node = (*node).clone();
            // Workaround for hosts sending mac addresses as nexthop - host will be assumed
            // to not have any uplink
            let mut set_nexthop = None;
            if let Some(nexthop) = &mut inner_node.gateway_nexthop {
                if nexthop.is_mac() {
                    log::debug!("Host {} has weird nexthop {}, using gateway", node.hostname, nexthop);
                    set_nexthop = Some(node.gateway.clone());
                }
            };
            if let Some(set_nexthop) = set_nexthop {
                inner_node.gateway_nexthop = set_nexthop;
            }
            let key = nodes.insert(NodeContainer {
                node: inner_node,
                uplink: None,
                downlinks: vec![]
            });

            id_lookup.insert(node.node_id.clone(), key);
            for addr in &node.addresses {
                ip_addrs.insert(addr.clone(), key);
            }
        }

        log::debug!("Graph building pass 2: building links");
        let mut downlinks = SecondaryMap::<NodeKey, Vec<NodeKey>>::new();
        for (key, node) in &mut nodes {
            if let Some(uplink) = &node.node.gateway_nexthop {
                let uplink_key = id_lookup.get(uplink)
                    .map(|key| *key)
                    .expect(&format!("ID {} not found", uplink));
                node.uplink = Some(uplink_key);

                if let Some(uplink_downlinks) = downlinks.get_mut(uplink_key) {
                    uplink_downlinks.push(key);
                } else {
                    downlinks.insert(uplink_key, vec![key]);
                }
            }
        }

        for (key, downlinks) in downlinks {
            if let Some(node) = nodes.get_mut(key) {
                node.downlinks = downlinks
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
                log::trace!(
                    "{} is version {} - marking as finished",
                    node.node.hostname,
                    node.node.firmware.release
                );
                policy = UpdatePolicy::Finished;
            } else {
                log::trace!("{} needs update", node.node.hostname);
                for downlink_key in &node.downlinks {
                    let downlink = nodes.get(*downlink_key).unwrap();
                    if downlink.node.firmware.release != config.latest_version {
                        if downlink.node.autoupdater.enabled {
                            log::trace!(
                                "{} has downlink {} which needs update first",
                                node.node.hostname,
                                downlink.node.hostname
                            );
                            policy = UpdatePolicy::Pending;
                        } else if !config.ignore_autoupdate_off {
                            log::trace!(
                                "{} has downlink {} which has autoupdate disabled. beeing carfule",
                                node.node.hostname,
                                downlink.node.hostname
                            );
                            policy = UpdatePolicy::Pending;
                        }
                    }
                }
            }

            log::debug!("Host {} has policy {:?}", node.node.hostname, policy);

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

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum UpdatePolicy {
    Pending,
    Ready,
    Finished
}