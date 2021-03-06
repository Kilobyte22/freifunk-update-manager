use crate::meshinfo::MeshInfo;
use slotmap::{DenseSlotMap, SecondaryMap};
use std::collections::HashMap;
use std::net::IpAddr;
use std::cmp;
use crate::config::SiteConfig;
use crate::persistence::PersistentState;
slotmap::new_key_type! { pub struct NodeKey; }

pub struct Graph {
    pub nodes: DenseSlotMap<NodeKey, NodeContainer>,
    pub ip_addrs: HashMap<IpAddr, NodeKey>,
    pub depths: SecondaryMap<NodeKey, u8>,
    pub max_depth: u8,
    pub deepest_node: Option<NodeKey>,
    pub update_policy: SecondaryMap<NodeKey, UpdatePolicy>,
}

impl Graph {
    pub fn build(info: &MeshInfo, config: &SiteConfig, persistent: &mut PersistentState) -> Graph {
        let mut nodes = DenseSlotMap::with_capacity_and_key(info.nodes.len());
        let mut id_lookup = HashMap::<crate::node_id::NodeID, NodeKey>::new();
        let mut ip_addrs = HashMap::new();

        let now = chrono::Utc::now();
        let node_cutoff_time = chrono::Duration::days(config.node_max_age_days as i64);

        log::debug!("Graph building pass 1: Setting up data");
        for node in &info.nodes {

            let inner_node = (*node).clone();

            if now - node.last_seen > node_cutoff_time {
                log::trace!(
                    "Node {} is offline for more than {} days, skipping",
                    node.hostname,
                    config.node_max_age_days
                );
                continue;
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

            let stored_uplink = persistent.link_history.get(&node.node.node_id);

            let nexthop = &node.node.gateway_nexthop
                .or_else(|| stored_uplink.map(|su| su.uplink));

            if let Some(uplink) = nexthop {
                let uplink_key = id_lookup.get(uplink)
                    .map(|key| *key);

                if let Some(uplink_key) = uplink_key {
                    node.uplink = Some(uplink_key);

                    if let Some(uplink_downlinks) = downlinks.get_mut(uplink_key) {
                        uplink_downlinks.push(key);
                    } else {
                        downlinks.insert(uplink_key, vec![key]);
                    }
                }
            }
        }

        for (key, downlinks) in downlinks {
            if let Some(node) = nodes.get_mut(key) {
                node.downlinks = downlinks
            }
        }

        let mut update_policy = SecondaryMap::new();
        log::debug!("Graph building pass 3: Factoring in if nodes have already received an update and failed at it");
        process_update_timeouts(
            &mut nodes,
            &mut update_policy,
            persistent,
            chrono::Duration::seconds(config.update_timeout as i64),
            config.broken_threshold as u32,
            &config.latest_version
        );

        log::debug!("Graph building pass 4: calculating node depth");
        let mut depths = SecondaryMap::with_capacity(nodes.len());

        let mut max_depth = 0;
        let mut deepest_node = None;

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
                        if d > max_depth {
                            max_depth = d;
                            deepest_node = Some(key);
                        }
                    }
                }
                if remove {
                    node_list.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        log::debug!("Graph building pass 5: determining per-node update policy");

        for (key, node) in &nodes {
            if update_policy.contains_key(key) {
                log::trace!(
                    "UpdatePolicy for {} has already been determined, not recalculating",
                    node.node.hostname
                );
                continue;
            }
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
                    let down_pol = update_policy.get(*downlink_key);
                    let update_override = down_pol == Some(&UpdatePolicy::Finished)
                        || down_pol == Some(&UpdatePolicy::Broken);
                    let firm_updated = downlink.node.firmware.release != config.latest_version;
                    if !update_override && firm_updated {
                        if downlink.node.autoupdater.enabled {
                            log::trace!(
                                "{} has downlink {} which needs update first",
                                node.node.hostname,
                                downlink.node.hostname
                            );
                            policy = UpdatePolicy::Pending;
                        } else if !config.ignore_autoupdate_off {
                            log::trace!(
                                "{} has downlink {} which has autoupdate disabled. being carful",
                                node.node.hostname,
                                downlink.node.hostname
                            );
                            policy = UpdatePolicy::Pending;
                        }
                    }
                }
            }

            log::trace!("Host {} has policy {:?}", node.node.hostname, policy);

            update_policy.insert(key, policy);
        }

        if let Some(deepest_node) = deepest_node {
            let node = nodes.get(deepest_node).unwrap();
            log::debug!("Deepest node is {} at a depth of {}", node.node.hostname, max_depth)
        }

        Graph {
            nodes,
            ip_addrs,
            depths,
            max_depth,
            deepest_node,
            update_policy
        }
    }
}

pub fn process_update_timeouts(
    nodes: &mut DenseSlotMap<NodeKey, NodeContainer>,
    update_policy: &mut SecondaryMap<NodeKey, UpdatePolicy>,
    pstate: &mut PersistentState,
    timeout: chrono::Duration,
    broken_threshold: u32,
    latest_fw: &str
) {
    let now = chrono::Utc::now();
    for (key, node) in nodes {
        if let Some(node_state) = pstate.node_state.get_mut(&node.node.node_id) {
            if let Some(updated_at) = node_state.update_received.clone() {
                // The host has recently been update
                if now - updated_at > timeout {
                    if node.node.is_online {
                        if node.node.firmware.release != latest_fw {
                            // Node has failed to update, increase counter
                            node_state.update_received = None;
                            node_state.update_attempts += 1;
                            log::trace!(
                                "Node {} has failed update {} times",
                                node.node.hostname,
                                node_state.update_attempts
                            );
                            if node_state.update_attempts >= broken_threshold {
                                update_policy.insert(key, UpdatePolicy::Broken);
                                log::warn!(
                                    "Node {} has failed update {} times and is now considered broken",
                                    node.node.hostname,
                                    node_state.update_attempts
                                );
                            } else {
                                update_policy.insert(key, UpdatePolicy::Ready);
                            }
                        }
                    } else {
                        log::trace!(
                            "Node {} gone offline for extended time, assuming it was successful",
                            node.node.hostname
                        );
                        // Node is still offline, assume it was successful
                        update_policy.insert(key, UpdatePolicy::Finished);
                    }
                }
            } else {
                if node_state.update_attempts >= broken_threshold {
                    update_policy.insert(key, UpdatePolicy::Broken);
                }
            }
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
    /// A Router cannot be updated yet, as it is waiting for downlinks to finish
    Pending,
    /// A router which can now be updated
    Ready,
    /// A router which is confirmed to be on latest version (either by timeout or active
    /// confirmation
    Finished,
    /// A router which has had multiple updates fail and will just be ignored
    Broken
}