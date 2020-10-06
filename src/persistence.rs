use std::collections::HashMap;
use crate::node_id::NodeID;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PersistentState {
    #[serde(default)]
    pub node_state: HashMap<NodeID, NodeState>,
    #[serde(default)]
    pub link_history: HashMap<NodeID, LinkInfo>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LinkInfo {
    pub uplink: NodeID,
    pub since: chrono::DateTime<chrono::Utc>
}

impl PersistentState {
    pub fn update_node(&mut self, name: &NodeID) {
        if let Some(node) = self.node_state.get_mut(name) {
            if node.update_received.is_none() {
                node.update_received = Some(chrono::offset::Utc::now());
            }
        } else {
            self.node_state.insert(name.clone(), NodeState {
                update_received: Some(chrono::offset::Utc::now()),
                .. NodeState::default()
            });
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct NodeState {
    pub update_received: Option<chrono::DateTime<chrono::offset::Utc>>,
    pub update_attempts: u32
}