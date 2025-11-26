/// RPC Service Definition for Distributed Image Cloud
///
/// This module defines the RPC interface for peer-to-peer communication
/// between cloud nodes using Tarpc framework.

use crate::messages::*;
use tarpc::context::Context;

/// Cloud Node RPC Service
///
/// This service defines all RPC methods that cloud nodes can call on each other.
/// Each method corresponds to a message type in the original UDP protocol.
#[tarpc::service]
pub trait CloudNodeService {
    /// Register a client session with a username
    async fn register_session(client_id: String, username: String)
        -> Result<bool, String>;

    /// Unregister a client session
    async fn unregister_session(client_id: String, username: String);

    /// Check if a username is available (not in use)
    async fn check_username_available(username: String) -> Result<bool, String>;

    /// Process an encryption request
    async fn encrypt_image(
        request_id: String,
        client_username: String,
        image_data: Vec<u8>,
        usernames: Vec<String>,
        quota: u32,
        forwarded: bool,
    ) -> Result<EncryptionResult, String>;

    /// Store an encrypted image for recipients
    async fn send_image(
        from_username: String,
        to_usernames: Vec<String>,
        encrypted_image: Vec<u8>,
        max_views: u32,
        image_id: String,
    ) -> Result<bool, String>;

    /// Query received images for a username
    async fn query_received_images(username: String)
        -> Result<Vec<ReceivedImageInfo>, String>;

    /// View an image (retrieves encrypted data and decrements quota)
    async fn view_image(username: String, image_id: String)
        -> Result<ViewImageResult, String>;

    // Peer-to-peer coordination methods

    /// Heartbeat message to indicate node is alive
    async fn heartbeat(node_id: u32, load: f32);

    /// Query current load of this node
    async fn query_load() -> f32;

    /// Election message (Bully algorithm)
    async fn election(candidate_id: u32) -> bool;

    /// Announce coordinator
    async fn announce_coordinator(coordinator_id: u32);

    /// Forward encryption request to this node
    async fn forward_request(
        request_id: String,
        client_username: String,
        image_data: Vec<u8>,
        usernames: Vec<String>,
        quota: u32,
        client_address: String,
    ) -> Result<EncryptionResult, String>;
}

/// Result types for RPC calls

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptionResult {
    pub success: bool,
    pub encrypted_image: Vec<u8>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ViewImageResult {
    pub image_data: Vec<u8>,
    pub remaining_views: u32,
}

/// Multicast helper for broadcasting RPC calls to all peers
pub struct RpcMulticaster {
    clients: Vec<CloudNodeServiceClient>,
}

impl RpcMulticaster {
    /// Create a new multicaster with connections to all peers
    pub fn new(clients: Vec<CloudNodeServiceClient>) -> Self {
        Self { clients }
    }

    /// Broadcast heartbeat to all peers
    pub async fn broadcast_heartbeat(&self, node_id: u32, load: f32) {
        let mut handles = vec![];

        for client in &self.clients {
            let client = client.clone();
            let handle = tokio::spawn(async move {
                let _ = client.heartbeat(Context::current(), node_id, load).await;
            });
            handles.push(handle);
        }

        // Wait for all broadcasts to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Broadcast coordinator announcement to all peers
    pub async fn broadcast_coordinator(&self, coordinator_id: u32) {
        let mut handles = vec![];

        for client in &self.clients {
            let client = client.clone();
            let handle = tokio::spawn(async move {
                let _ = client.announce_coordinator(Context::current(), coordinator_id).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Query load from all peers and return results
    pub async fn query_all_loads(&self) -> Vec<(usize, f32)> {
        let mut handles = vec![];

        for (idx, client) in self.clients.iter().enumerate() {
            let client = client.clone();
            let handle = tokio::spawn(async move {
                let load = client.query_load(Context::current()).await.unwrap_or(100.0);
                (idx, load)
            });
            handles.push(handle);
        }

        let mut results = vec![];
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }

        results
    }
}
