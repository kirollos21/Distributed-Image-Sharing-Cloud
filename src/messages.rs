use serde::{Deserialize, Serialize};
use std::fmt;

/// Node ID for cloud nodes
pub type NodeId = u8;

/// Information about a received image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedImageInfo {
    pub image_id: String,
    pub from_username: String,
    pub remaining_views: u8,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteInfo {
    pub note_id: String,
    pub from_id: u8,
    pub from_username: String,
    pub content: String,
    pub timestamp: i64,
}

/// User status enum (for client-facing messages)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientUserStatus {
    Online,
    Offline,
    Idle,
}

/// User info returned to clients (no password)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientUserInfo {
    pub id: String,
    pub username: String,
    pub status: ClientUserStatus,
    pub last_seen: i64,
    pub ip: String,
    pub gallery: Vec<String>,
}

/// Message types exchanged between nodes and clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // Election messages (Bully Algorithm)
    Election { from_node: NodeId },
    Ok { from_node: NodeId },
    Coordinator { node_id: NodeId, load: f64 },

    // Forwarding wrapper - wraps any message with original client address
    ForwardedMessage {
        original_message: Box<Message>,
        client_address: String, // Original client's address for direct response
    },

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
        quota: u8,
        forwarded: bool, // Prevent infinite forwarding loops
        client_address: Option<String>, // Original client address for direct response
    },
    DecryptionRequest {
        request_id: String,
        client_username: String,
        encrypted_image: Vec<u8>,
        usernames: Vec<String>,
        quota: u8,
    },

    // Response messages
    EncryptionResponse {
        request_id: String,
        encrypted_image: Vec<u8>,
        success: bool,
        error: Option<String>,
    },
    DecryptionResponse {
        request_id: String,
        decrypted_image: Vec<u8>,
        success: bool,
        error: Option<String>,
    },

    // Load query for election
    LoadQuery { from_node: NodeId },
    LoadResponse { 
        node_id: NodeId, 
        load: f64, 
        queue_length: usize,
        processed_count: usize, // Total requests processed by this node
    },

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

    // Heartbeat (includes load info to reduce overhead)
    Heartbeat {
        from_node: NodeId,
        load: f64,
        processed_count: usize,
    },
    HeartbeatAck {
        from_node: NodeId,
        load: f64,
        processed_count: usize,
    },

    // Image sending/receiving messages
    SendImage {
        from_username: String,
        to_usernames: Vec<String>,
        encrypted_image: Vec<u8>,
        max_views: u8,
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
        remaining_views: Option<u8>,
        error: Option<String>,
    },
    CheckUsernameAvailable {
        username: String,
    },
    CheckUsernameAvailableResponse {
        username: String,
        is_available: bool,
    },
    SendNote {
        note_id: String,
        from_id: u8,
        to_id: u8,
        from_username: String,
        content: String,
        timestamp: i64,
    },
    SendNoteResponse {
        success: bool,
        note_id: String,
        error: Option<String>,
    },
    GetPendingNotes {
        user_id: u8,
    },
    GetPendingNotesResponse {
        notes: Vec<NoteInfo>,
    },

    // Client authentication & presence (node handles Firebase)
    ClientLogin {
        user_id: String,
        password: String,
        client_ip: String,
    },
    ClientLoginResponse {
        success: bool,
        user_info: Option<ClientUserInfo>,
        error: Option<String>,
    },
    ClientHeartbeat {
        user_id: String,
    },
    ClientLogout {
        user_id: String,
    },

    // User browsing (node fetches from Firebase)
    GetUserList,
    GetUserListResponse {
        users: Vec<ClientUserInfo>,
    },
    GetUserInfo {
        user_id: String,
    },
    GetUserInfoResponse {
        success: bool,
        user_info: Option<ClientUserInfo>,
        error: Option<String>,
    },
    GetUserGallery {
        user_id: String,
    },
    GetUserGalleryResponse {
        success: bool,
        images: Vec<String>,
        error: Option<String>,
    },
    
    // Image request messages
    RequestImage {
        request_id: String,
        from_user_id: String,
        from_username: String,
        to_user_id: String,
        to_username: String,
        image_index: usize,  // Index in gallery (0-4)
        timestamp: i64,
        quota: u8,  // View quota (1-99)
    },
    RequestImageResponse {
        success: bool,
        request_id: String,
        error: Option<String>,
    },
    GetImageRequests {
        user_id: String,
    },
    GetImageRequestsResponse {
        incoming: Vec<ImageRequestInfo>,  // Requests TO you
        outgoing: Vec<ImageRequestInfo>,  // Requests FROM you
    },
    RespondToImageRequest {
        request_id: String,
        user_id: String,  // User responding
        accepted: bool,
    },
    RespondToImageRequestResponse {
        success: bool,
        error: Option<String>,
    },
    DeleteImageRequest {
        request_id: String,
        user_id: String,  // User deleting
    },
    DeleteImageRequestResponse {
        success: bool,
        error: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequestInfo {
    pub request_id: String,
    pub from_user_id: String,
    pub from_username: String,
    pub to_user_id: String,
    pub to_username: String,
    pub image_index: usize,
    pub timestamp: i64,
    pub status: RequestStatus,  // pending, accepted, rejected
    pub quota: u8,  // View quota (1-99)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestStatus {
    Pending,
    Accepted,
    Rejected,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Election { from_node } => write!(f, "ELECTION from Node {}", from_node),
            Message::Ok { from_node } => write!(f, "OK from Node {}", from_node),
            Message::Coordinator { node_id, load } => {
                write!(f, "COORDINATOR Node {} (load: {:.2})", node_id, load)
            }
            Message::ForwardedMessage { client_address, .. } => {
                write!(f, "FORWARDED_MESSAGE from client {}", client_address)
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
            Message::DecryptionRequest { request_id, .. } => {
                write!(f, "DECRYPTION_REQUEST {}", request_id)
            }
            Message::EncryptionResponse { request_id, success, .. } => {
                write!(f, "ENCRYPTION_RESPONSE {} (success: {})", request_id, success)
            }
            Message::DecryptionResponse { request_id, success, .. } => {
                write!(f, "DECRYPTION_RESPONSE {} (success: {})", request_id, success)
            }
            Message::LoadQuery { from_node } => write!(f, "LOAD_QUERY from Node {}", from_node),
            Message::LoadResponse { node_id, load, queue_length, processed_count } => {
                write!(f, "LOAD_RESPONSE from Node {} (load: {:.2}, queue: {}, processed: {})", 
                       node_id, load, queue_length, processed_count)
            }
            Message::StateSync { from_node } => write!(f, "STATE_SYNC from Node {}", from_node),
            Message::StateSyncResponse { coordinator_id, .. } => {
                write!(f, "STATE_SYNC_RESPONSE (coordinator: {})", coordinator_id)
            }
            Message::CoordinatorQuery => write!(f, "COORDINATOR_QUERY"),
            Message::CoordinatorQueryResponse { coordinator_address } => {
                write!(f, "COORDINATOR_QUERY_RESPONSE (address: {})", coordinator_address)
            }
            Message::Heartbeat { from_node, load, processed_count } => {
                write!(f, "HEARTBEAT from Node {} (load: {:.2}, processed: {})", from_node, load, processed_count)
            }
            Message::HeartbeatAck { from_node, load, processed_count } => {
                write!(f, "HEARTBEAT_ACK from Node {} (load: {:.2}, processed: {})", from_node, load, processed_count)
            }
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
            Message::SendNote { note_id, from_username, to_id, .. } => {
                write!(f, "SEND_NOTE {} from {} to {}", note_id, from_username, to_id)
            }
            Message::SendNoteResponse { success, note_id, .. } => {
                write!(f, "SEND_NOTE_RESPONSE {} (success: {})", note_id, success)
            }
            Message::GetPendingNotes { user_id } => {
                write!(f, "GET_PENDING_NOTES for User {}", user_id)
            }
            Message::GetPendingNotesResponse { notes } => {
                write!(f, "GET_PENDING_NOTES_RESPONSE ({} notes)", notes.len())
            }
            Message::ClientLogin { user_id, .. } => {
                write!(f, "CLIENT_LOGIN user_id: {}", user_id)
            }
            Message::ClientLoginResponse { success, .. } => {
                write!(f, "CLIENT_LOGIN_RESPONSE (success: {})", success)
            }
            Message::ClientHeartbeat { user_id } => {
                write!(f, "CLIENT_HEARTBEAT user_id: {}", user_id)
            }
            Message::ClientLogout { user_id } => {
                write!(f, "CLIENT_LOGOUT user_id: {}", user_id)
            }
            Message::GetUserList => {
                write!(f, "GET_USER_LIST")
            }
            Message::GetUserListResponse { users } => {
                write!(f, "GET_USER_LIST_RESPONSE ({} users)", users.len())
            }
            Message::GetUserInfo { user_id } => {
                write!(f, "GET_USER_INFO user_id: {}", user_id)
            }
            Message::GetUserInfoResponse { success, .. } => {
                write!(f, "GET_USER_INFO_RESPONSE (success: {})", success)
            }
            Message::GetUserGallery { user_id } => {
                write!(f, "GET_USER_GALLERY user_id: {}", user_id)
            }
            Message::GetUserGalleryResponse { success, images, .. } => {
                write!(f, "GET_USER_GALLERY_RESPONSE (success: {}, {} images)", success, images.len())
            }
            Message::RequestImage { request_id, from_username, to_username, image_index, quota, .. } => {
                write!(f, "REQUEST_IMAGE {} from {} to {} (image #{}, quota: {})", request_id, from_username, to_username, image_index, quota)
            }
            Message::RequestImageResponse { success, request_id, .. } => {
                write!(f, "REQUEST_IMAGE_RESPONSE {} (success: {})", request_id, success)
            }
            Message::GetImageRequests { user_id } => {
                write!(f, "GET_IMAGE_REQUESTS user_id: {}", user_id)
            }
            Message::GetImageRequestsResponse { incoming, outgoing } => {
                write!(f, "GET_IMAGE_REQUESTS_RESPONSE ({} incoming, {} outgoing)", incoming.len(), outgoing.len())
            }
            Message::RespondToImageRequest { request_id, accepted, .. } => {
                write!(f, "RESPOND_TO_IMAGE_REQUEST {} (accepted: {})", request_id, accepted)
            }
            Message::RespondToImageRequestResponse { success, .. } => {
                write!(f, "RESPOND_TO_IMAGE_REQUEST_RESPONSE (success: {})", success)
            }
            Message::DeleteImageRequest { request_id, .. } => {
                write!(f, "DELETE_IMAGE_REQUEST {}", request_id)
            }
            Message::DeleteImageRequestResponse { success, .. } => {
                write!(f, "DELETE_IMAGE_REQUEST_RESPONSE (success: {})", success)
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
