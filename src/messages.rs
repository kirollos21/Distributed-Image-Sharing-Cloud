use serde::{Deserialize, Serialize};
use std::fmt;

/// Node ID for cloud nodes
pub type NodeId = u32;

/// Information about a received image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedImageInfo {
    pub image_id: String,
    pub from_username: String,
    pub remaining_views: u32,
    pub timestamp: i64,
}

/// Message types exchanged between nodes and clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // Election messages (Bully Algorithm)
    Election { from_node: NodeId },
    Ok { from_node: NodeId },
    Coordinator { node_id: NodeId, load: f64 },

    // Session management messages
    SessionRegister {
        client_id: String,
        username: String,
    },
    SessionRegisterResponse {
        success: bool,
        error: Option<String>,
    },
    SessionUnregister {
        client_id: String,
        username: String,
    },

    // Client request messages
    EncryptionRequest {
        request_id: String,
        client_username: String,
        image_data: Vec<u8>,
        usernames: Vec<String>,
        quota: u32,
        forwarded: bool, // Prevent infinite forwarding loops
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

    // Coordinator query (for clients)
    CoordinatorQuery,
    CoordinatorQueryResponse {
        coordinator_address: String,
    },

    // Heartbeat
    Heartbeat { from_node: NodeId },
    HeartbeatAck { from_node: NodeId },

    // Image sending/receiving messages
    SendImage {
        from_username: String,
        to_usernames: Vec<String>,
        encrypted_image: Vec<u8>,
        max_views: u32,
        image_id: String,
    },
    SendImageResponse {
        success: bool,
        image_id: String,
        error: Option<String>,
    },
    QueryReceivedImages {
        username: String,
    },
    QueryReceivedImagesResponse {
        images: Vec<ReceivedImageInfo>,
    },
    ViewImage {
        username: String,
        image_id: String,
    },
    ViewImageResponse {
        success: bool,
        image_data: Option<Vec<u8>>,
        remaining_views: Option<u32>,
        error: Option<String>,
    },
    CheckUsernameAvailable {
        username: String,
    },
    CheckUsernameAvailableResponse {
        username: String,
        is_available: bool,
    },
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Election { from_node } => write!(f, "ELECTION from Node {}", from_node),
            Message::Ok { from_node } => write!(f, "OK from Node {}", from_node),
            Message::Coordinator { node_id, load } => {
                write!(f, "COORDINATOR Node {} (load: {:.2})", node_id, load)
            }
            Message::SessionRegister { username, .. } => {
                write!(f, "SESSION_REGISTER username: {}", username)
            }
            Message::SessionRegisterResponse { success, .. } => {
                write!(f, "SESSION_REGISTER_RESPONSE (success: {})", success)
            }
            Message::SessionUnregister { username, .. } => {
                write!(f, "SESSION_UNREGISTER username: {}", username)
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
            Message::CoordinatorQuery => write!(f, "COORDINATOR_QUERY"),
            Message::CoordinatorQueryResponse { coordinator_address } => {
                write!(f, "COORDINATOR_QUERY_RESPONSE (address: {})", coordinator_address)
            }
            Message::Heartbeat { from_node } => write!(f, "HEARTBEAT from Node {}", from_node),
            Message::HeartbeatAck { from_node } => write!(f, "HEARTBEAT_ACK from Node {}", from_node),
            Message::SendImage { from_username, to_usernames, image_id, .. } => {
                write!(f, "SEND_IMAGE {} from {} to {:?}", image_id, from_username, to_usernames)
            }
            Message::SendImageResponse { success, image_id, .. } => {
                write!(f, "SEND_IMAGE_RESPONSE {} (success: {})", image_id, success)
            }
            Message::QueryReceivedImages { username } => {
                write!(f, "QUERY_RECEIVED_IMAGES for {}", username)
            }
            Message::QueryReceivedImagesResponse { images } => {
                write!(f, "QUERY_RECEIVED_IMAGES_RESPONSE ({} images)", images.len())
            }
            Message::ViewImage { username, image_id } => {
                write!(f, "VIEW_IMAGE {} by {}", image_id, username)
            }
            Message::ViewImageResponse { success, remaining_views, .. } => {
                write!(f, "VIEW_IMAGE_RESPONSE (success: {}, remaining: {:?})", success, remaining_views)
            }
            Message::CheckUsernameAvailable { username } => {
                write!(f, "CHECK_USERNAME_AVAILABLE {}", username)
            }
            Message::CheckUsernameAvailableResponse { username, is_available } => {
                write!(f, "CHECK_USERNAME_AVAILABLE_RESPONSE {} (available: {})", username, is_available)
            }
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
