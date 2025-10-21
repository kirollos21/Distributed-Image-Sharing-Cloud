use serde::{Deserialize, Serialize};
use std::fmt;

/// Node ID for cloud nodes
pub type NodeId = u32;

/// Message types exchanged between nodes and clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // Election messages (Bully Algorithm)
    Election { from_node: NodeId },
    Ok { from_node: NodeId },
    Coordinator { node_id: NodeId, load: f64 },

    // Client request messages
    EncryptionRequest {
        request_id: String,
        image_data: Vec<u8>,
        usernames: Vec<String>,
        quota: u32,
    },

    // Response messages
    EncryptionResponse {
        request_id: String,
        encrypted_image: Vec<u8>,
        success: bool,
        error: Option<String>,
    },

    // Load query for election
    LoadQuery { from_node: NodeId },
    LoadResponse { node_id: NodeId, load: f64, queue_length: usize },

    // State synchronization
    StateSync { from_node: NodeId },
    StateSyncResponse {
        coordinator_id: NodeId,
        load_metrics: Vec<(NodeId, f64)>,
        timestamp: i64,
    },

    // Heartbeat
    Heartbeat { from_node: NodeId },
    HeartbeatAck { from_node: NodeId },
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Election { from_node } => write!(f, "ELECTION from Node {}", from_node),
            Message::Ok { from_node } => write!(f, "OK from Node {}", from_node),
            Message::Coordinator { node_id, load } => {
                write!(f, "COORDINATOR Node {} (load: {:.2})", node_id, load)
            }
            Message::EncryptionRequest { request_id, .. } => {
                write!(f, "ENCRYPTION_REQUEST {}", request_id)
            }
            Message::EncryptionResponse { request_id, success, .. } => {
                write!(f, "ENCRYPTION_RESPONSE {} (success: {})", request_id, success)
            }
            Message::LoadQuery { from_node } => write!(f, "LOAD_QUERY from Node {}", from_node),
            Message::LoadResponse { node_id, load, queue_length } => {
                write!(f, "LOAD_RESPONSE Node {} (load: {:.2}, queue: {})", node_id, load, queue_length)
            }
            Message::StateSync { from_node } => write!(f, "STATE_SYNC from Node {}", from_node),
            Message::StateSyncResponse { coordinator_id, .. } => {
                write!(f, "STATE_SYNC_RESPONSE (coordinator: {})", coordinator_id)
            }
            Message::Heartbeat { from_node } => write!(f, "HEARTBEAT from Node {}", from_node),
            Message::HeartbeatAck { from_node } => write!(f, "HEARTBEAT_ACK from Node {}", from_node),
        }
    }
}

/// Node state enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    Active,
    Failed,
    Recovering,
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeState::Active => write!(f, "ACTIVE"),
            NodeState::Failed => write!(f, "FAILED"),
            NodeState::Recovering => write!(f, "RECOVERING"),
        }
    }
}
