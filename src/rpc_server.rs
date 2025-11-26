/// RPC Server Implementation for Cloud Nodes
///
/// This module implements the RPC server that handles incoming RPC requests
/// from clients and other nodes using the Tarpc framework.

use crate::messages::*;
use crate::node::CloudNode;
use crate::rpc_service::*;
use log::info;
use std::net::SocketAddr;
use std::sync::Arc;
use tarpc::{
    context::Context,
    server::{self, Channel},
};
use tokio::net::ToSocketAddrs;

/// RPC Server implementation that wraps CloudNode
#[derive(Clone)]
pub struct CloudNodeRpcServer {
    node: Arc<CloudNode>,
}

impl CloudNodeRpcServer {
    pub fn new(node: Arc<CloudNode>) -> Self {
        Self { node }
    }
}

/// Implementation of the CloudNodeService trait for RPC
#[tarpc::server]
impl CloudNodeService for CloudNodeRpcServer {
    async fn register_session(
        self,
        _: Context,
        client_id: String,
        username: String,
    ) -> Result<bool, String> {
        info!(
            "[Node {}] RPC: register_session({}, {})",
            self.node.id, client_id, username
        );

        // Create the message and process it
        let message = Message::SessionRegister {
            client_id,
            username,
        };

        match self.node.process_message(message).await {
            Some(Message::SessionRegisterResponse { success, error }) => {
                if success {
                    Ok(true)
                } else {
                    Err(error.unwrap_or_else(|| "Registration failed".to_string()))
                }
            }
            _ => Err("Unexpected response".to_string()),
        }
    }

    async fn unregister_session(self, _: Context, client_id: String, username: String) {
        info!(
            "[Node {}] RPC: unregister_session({}, {})",
            self.node.id, client_id, username
        );

        let message = Message::SessionUnregister {
            client_id,
            username,
        };

        self.node.process_message(message).await;
    }

    async fn check_username_available(
        self,
        _: Context,
        username: String,
    ) -> Result<bool, String> {
        info!(
            "[Node {}] RPC: check_username_available({})",
            self.node.id, username
        );

        let message = Message::CheckUsernameAvailable { username };

        match self.node.process_message(message).await {
            Some(Message::CheckUsernameAvailableResponse { username: _, is_available }) => Ok(is_available),
            _ => Err("Unexpected response".to_string()),
        }
    }

    async fn encrypt_image(
        self,
        _: Context,
        request_id: String,
        client_username: String,
        image_data: Vec<u8>,
        usernames: Vec<String>,
        quota: u32,
        forwarded: bool,
    ) -> Result<EncryptionResult, String> {
        info!(
            "[Node {}] RPC: encrypt_image({}, {} bytes, {} users)",
            self.node.id,
            request_id,
            image_data.len(),
            usernames.len()
        );

        let message = Message::EncryptionRequest {
            request_id,
            client_username,
            image_data,
            usernames,
            quota,
            forwarded,
            client_address: None,
        };

        match self.node.process_message(message).await {
            Some(Message::EncryptionResponse {
                encrypted_image,
                success,
                error,
                ..
            }) => Ok(EncryptionResult {
                success,
                encrypted_image,
                error,
            }),
            _ => Err("Unexpected response".to_string()),
        }
    }

    async fn send_image(
        self,
        _: Context,
        from_username: String,
        to_usernames: Vec<String>,
        encrypted_image: Vec<u8>,
        max_views: u32,
        image_id: String,
    ) -> Result<bool, String> {
        info!(
            "[Node {}] RPC: send_image({}, {} recipients, {} bytes)",
            self.node.id,
            from_username,
            to_usernames.len(),
            encrypted_image.len()
        );

        let message = Message::SendImage {
            from_username,
            to_usernames,
            encrypted_image,
            max_views,
            image_id,
        };

        match self.node.process_message(message).await {
            Some(Message::SendImageResponse { success, error, .. }) => {
                if success {
                    Ok(true)
                } else {
                    Err(error.unwrap_or_else(|| "Send failed".to_string()))
                }
            }
            _ => Err("Unexpected response".to_string()),
        }
    }

    async fn query_received_images(
        self,
        _: Context,
        username: String,
    ) -> Result<Vec<ReceivedImageInfo>, String> {
        info!(
            "[Node {}] RPC: query_received_images({})",
            self.node.id, username
        );

        let message = Message::QueryReceivedImages { username };

        match self.node.process_message(message).await {
            Some(Message::QueryReceivedImagesResponse { images }) => Ok(images),
            _ => Err("Unexpected response".to_string()),
        }
    }

    async fn view_image(
        self,
        _: Context,
        username: String,
        image_id: String,
    ) -> Result<ViewImageResult, String> {
        info!(
            "[Node {}] RPC: view_image({}, {})",
            self.node.id, username, image_id
        );

        let message = Message::ViewImage {
            username,
            image_id,
        };

        match self.node.process_message(message).await {
            Some(Message::ViewImageResponse {
                success,
                image_data,
                remaining_views,
                error,
            }) => {
                if success {
                    Ok(ViewImageResult {
                        image_data: image_data.unwrap_or_default(),
                        remaining_views: remaining_views.unwrap_or(0),
                    })
                } else {
                    Err(error.unwrap_or_else(|| "View failed".to_string()))
                }
            }
            _ => Err("Unexpected response".to_string()),
        }
    }

    // Peer-to-peer coordination methods

    async fn heartbeat(self, _: Context, node_id: u32, load: f32) {
        // Process heartbeat
        self.node.handle_peer_heartbeat(node_id, load).await;
    }

    async fn query_load(self, _: Context) -> f32 {
        self.node.get_current_load().await
    }

    async fn election(self, _: Context, candidate_id: u32) -> bool {
        info!(
            "[Node {}] RPC: election from Node {}",
            self.node.id, candidate_id
        );
        self.node.handle_election_request(candidate_id).await
    }

    async fn announce_coordinator(self, _: Context, coordinator_id: u32) {
        info!(
            "[Node {}] RPC: coordinator announcement - Node {}",
            self.node.id, coordinator_id
        );
        self.node.update_coordinator(coordinator_id).await;
    }

    async fn forward_request(
        self,
        _: Context,
        request_id: String,
        client_username: String,
        image_data: Vec<u8>,
        usernames: Vec<String>,
        quota: u32,
        client_address: String,
    ) -> Result<EncryptionResult, String> {
        info!(
            "[Node {}] RPC: forward_request({})",
            self.node.id, request_id
        );

        let message = Message::EncryptionRequest {
            request_id,
            client_username,
            image_data,
            usernames,
            quota,
            forwarded: true,
            client_address: Some(client_address),
        };

        match self.node.process_message(message).await {
            Some(Message::EncryptionResponse {
                encrypted_image,
                success,
                error,
                ..
            }) => Ok(EncryptionResult {
                success,
                encrypted_image,
                error,
            }),
            _ => Err("Unexpected response".to_string()),
        }
    }
}

/// Start the RPC server for a cloud node
pub async fn start_rpc_server<A>(
    node: Arc<CloudNode>,
    addr: A,
) -> Result<(), Box<dyn std::error::Error>>
where
    A: ToSocketAddrs,
{
    let server = CloudNodeRpcServer::new(node.clone());

    info!("[Node {}] Starting RPC server...", node.id);

    // Create TCP listener with JSON codec
    let listener = tarpc::serde_transport::tcp::listen(addr, tokio_serde::formats::Json::default)
        .await
        .map_err(|e| format!("Failed to start RPC listener: {}", e))?;

    info!(
        "[Node {}] RPC server listening on {}",
        node.id,
        listener.local_addr()
    );

    // Accept connections in a loop
    listener
        .filter_map(|r| async move { r.ok() })
        .map(server::BaseChannel::with_defaults)
        .max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        .map(|channel| {
            let server = server.clone();
            async move {
                channel.execute(server.serve()).await;
            }
        })
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;

    Ok(())
}

/// Helper to create RPC client connection to a peer node
pub async fn connect_to_peer(
    peer_addr: SocketAddr,
) -> Result<CloudNodeServiceClient, Box<dyn std::error::Error>> {
    info!("Connecting to peer at {}", peer_addr);

    let transport = tarpc::serde_transport::tcp::connect(peer_addr, tokio_serde::formats::Json::default).await?;
    let client = CloudNodeServiceClient::new(tarpc::client::Config::default(), transport).spawn();

    Ok(client)
}
