use std::collections::HashMap;
use crate::meshinfo::NodeID;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PersistentState {
    pub node_state: HashMap<NodeID, NodeState>
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