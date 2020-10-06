use crate::meshinfo::NodeID;

use serde::Serialize;
use crate::MainState;
use std::collections::HashMap;
use crate::graph::UpdatePolicy;

#[derive(Serialize, Default)]
pub struct SiteDump {
    counts: NodeCounts,
    updated: Vec<NodeInfo>,
    pending: Vec<NodeInfo>,
    failed: Vec<NodeInfo>,
    scheduled: Vec<NodeInfo>,
    broken: Vec<NodeInfo>
}

#[derive(Serialize)]
struct NodeInfo {
    id: NodeID,
    hostname: String,
    update_fail_count: u32,
    updated_at: Option<chrono::DateTime<chrono::Utc>>
}

#[derive(Serialize, Default)]
struct NodeCounts {
    updated: u32,
    pending: u32,
    failed: u32,
    scheduled: u32,
    broken: u32
}

pub async fn generate(state: &MainState) -> HashMap<String, SiteDump> {
    let mut ret = HashMap::new();
    for ((site_name, branch), site) in &state.graphs {
        let mut site_ret = SiteDump::default();
        let graph = site.graph.read().await;
        let persistent = site.persistent.lock().await;
        for (key, node) in &graph.nodes {
            let node_state = persistent.node_state.get(&node.node.node_id);
            let info = NodeInfo {
                id: node.node.node_id.clone(),
                hostname: node.node.hostname.clone(),
                update_fail_count: node_state.map(|s| s.update_attempts).unwrap_or(0),
                updated_at: node_state.and_then(|s| s.update_received)
            };
            match graph.update_policy.get(key) {
                Some(UpdatePolicy::Ready) => {
                    if info.update_fail_count > 0 {
                        site_ret.failed.push(info);
                    } else {
                        site_ret.scheduled.push(info);
                    }
                },
                Some(UpdatePolicy::Finished) => {
                    site_ret.updated.push(info);
                },
                Some(UpdatePolicy::Broken) => {
                    site_ret.broken.push(info);
                },
                Some(UpdatePolicy::Pending) => {
                    site_ret.pending.push(info);
                },
                None => {
                    log::warn!(
                        "Node {} does not have update policy",
                        info.hostname
                    );
                }
            }
        }
        site_ret.counts = NodeCounts {
            updated: site_ret.updated.len() as u32,
            pending: site_ret.pending.len() as u32,
            failed: site_ret.failed.len() as u32,
            scheduled: site_ret.scheduled.len() as u32,
            broken: site_ret.broken.len() as u32
        };
        ret.insert(format!("{}_{}", site_name, branch), site_ret);
    }
    ret
}