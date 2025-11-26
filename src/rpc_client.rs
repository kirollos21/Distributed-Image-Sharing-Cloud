/// RPC Client Wrapper for easy RPC communication with cloud nodes
///
/// This module provides a high-level client interface for communicating
/// with cloud nodes using RPC instead of raw UDP.

use crate::messages::*;
use crate::rpc_service::*;
use log::{error, info, warn};
use std::net::SocketAddr;
use std::str::FromStr;
use tarpc::context;

/// RPC-based client for distributed image cloud
pub struct RpcClient {
    pub id: usize,
    pub cloud_addresses: Vec<String>,
}

impl RpcClient {
    pub fn new(id: usize, cloud_addresses: Vec<String>) -> Self {
        Self {
            id,
            cloud_addresses,
        }
    }

    /// Connect to a node and get an RPC client
    async fn connect_to_node(
        &self,
        address: &str,
    ) -> Result<CloudNodeServiceClient, String> {
        let sock_addr = SocketAddr::from_str(address)
            .map_err(|e| format!("Invalid address {}: {}", address, e))?;

        let transport = tarpc::serde_transport::tcp::connect(sock_addr, tokio_serde::formats::Json::default)
            .await
            .map_err(|e| format!("Failed to connect to {}: {}", address, e))?;

        let client =
            CloudNodeServiceClient::new(tarpc::client::Config::default(), transport).spawn();

        Ok(client)
    }

    /// Register a session with username
    pub async fn register_session(
        &self,
        client_id: String,
        username: String,
    ) -> Result<(), String> {
        info!(
            "[RpcClient {}] Registering username: {}",
            self.id, username
        );

        for address in &self.cloud_addresses {
            match self.connect_to_node(address).await {
                Ok(client) => {
                    match client
                        .register_session(context::current(), client_id.clone(), username.clone())
                        .await
                    {
                        Ok(Ok(true)) => {
                            info!(
                                "[RpcClient {}] Successfully registered with {}",
                                self.id, address
                            );
                            return Ok(());
                        }
                        Ok(Err(e)) => {
                            error!("[RpcClient {}] Registration rejected: {}", self.id, e);
                            return Err(e);
                        }
                        Err(e) => {
                            warn!(
                                "[RpcClient {}] RPC error with {}: {}",
                                self.id, address, e
                            );
                            continue;
                        }
                        Ok(Ok(false)) => {
                            return Err("Registration failed".to_string());
                        }
                    }
                }
                Err(e) => {
                    warn!("[RpcClient {}] Failed to connect to {}: {}", self.id, address, e);
                    continue;
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Unregister a session
    pub async fn unregister_session(&self, client_id: String, username: String) {
        info!(
            "[RpcClient {}] Unregistering username: {}",
            self.id, username
        );

        for address in &self.cloud_addresses {
            if let Ok(client) = self.connect_to_node(address).await {
                let _ = client
                    .unregister_session(context::current(), client_id.clone(), username.clone())
                    .await;
            }
        }
    }

    /// Check if username is available
    pub async fn check_username_available(&self, username: String) -> Result<bool, String> {
        for address in &self.cloud_addresses {
            match self.connect_to_node(address).await {
                Ok(client) => {
                    match client
                        .check_username_available(context::current(), username.clone())
                        .await
                    {
                        Ok(Ok(available)) => return Ok(available),
                        Ok(Err(e)) => return Err(e),
                        Err(e) => {
                            warn!(
                                "[RpcClient {}] RPC error with {}: {}",
                                self.id, address, e
                            );
                            continue;
                        }
                    }
                }
                Err(e) => {
                    warn!("[RpcClient {}] Failed to connect to {}: {}", self.id, address, e);
                    continue;
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Send encryption request
    pub async fn send_encryption_request(
        &self,
        request_id: String,
        client_username: String,
        image_data: Vec<u8>,
        usernames: Vec<String>,
        quota: u32,
    ) -> Result<Message, String> {
        info!(
            "[RpcClient {}] Sending encryption request: {}",
            self.id, request_id
        );

        // Multicast to all nodes and return first successful response
        let mut handles = vec![];

        for address in &self.cloud_addresses {
            let address = address.clone();
            let request_id = request_id.clone();
            let client_username = client_username.clone();
            let image_data = image_data.clone();
            let usernames = usernames.clone();
            let client_id = self.id;

            let handle = tokio::spawn(async move {
                let sock_addr = match SocketAddr::from_str(&address) {
                    Ok(addr) => addr,
                    Err(_) => return None,
                };
                let transport =match tarpc::serde_transport::tcp::connect(sock_addr, tokio_serde::formats::Json::default)
                        .await {
                    Ok(t) => t,
                    Err(_) => return None,
                };
                let client = CloudNodeServiceClient::new(
                    tarpc::client::Config::default(),
                    transport,
                )
                .spawn();

                client
                    .encrypt_image(
                        context::current(),
                        request_id,
                        client_username,
                        image_data,
                        usernames,
                        quota,
                        false,
                    )
                    .await
                    .ok()
            });

            handles.push(handle);
        }

        // Wait for first successful response
        for handle in handles {
            if let Ok(Some(Ok(result))) = handle.await {
                if result.success {
                    return Ok(Message::EncryptionResponse {
                        request_id: String::new(),
                        encrypted_image: result.encrypted_image,
                        success: true,
                        error: None,
                    });
                }
            }
        }

        Err("All nodes failed to respond".to_string())
    }

    /// Send image to users
    pub async fn send_image(
        &self,
        from_username: String,
        to_usernames: Vec<String>,
        encrypted_image: Vec<u8>,
        max_views: u32,
        image_id: String,
    ) -> Result<String, String> {
        for address in &self.cloud_addresses {
            match self.connect_to_node(address).await {
                Ok(client) => {
                    match client
                        .send_image(
                            context::current(),
                            from_username.clone(),
                            to_usernames.clone(),
                            encrypted_image.clone(),
                            max_views,
                            image_id.clone(),
                        )
                        .await
                    {
                        Ok(Ok(true)) => {
                            info!(
                                "[RpcClient {}] Successfully sent image: {}",
                                self.id, image_id
                            );
                            return Ok(image_id);
                        }
                        Ok(Err(e)) => return Err(e),
                        Err(e) => {
                            warn!("[RpcClient {}] RPC error: {}", self.id, e);
                            continue;
                        }
                        Ok(Ok(false)) => {
                            return Err("Send failed".to_string());
                        }
                    }
                }
                Err(e) => {
                    warn!("[RpcClient {}] Connection error: {}", self.id, e);
                    continue;
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Query received images for a username
    /// Queries ALL nodes and merges results
    pub async fn query_received_images(
        &self,
        username: String,
    ) -> Result<Vec<ReceivedImageInfo>, String> {
        info!(
            "[RpcClient {}] Querying received images for: {} from ALL nodes",
            self.id, username
        );

        let mut handles = vec![];

        for address in &self.cloud_addresses {
            let address = address.clone();
            let username = username.clone();

            let handle = tokio::spawn(async move {
                let sock_addr = match SocketAddr::from_str(&address) {
                    Ok(addr) => addr,
                    Err(_) => return None,
                };
                let transport = match tarpc::serde_transport::tcp::connect(sock_addr, tokio_serde::formats::Json::default)
                        .await {
                    Ok(t) => t,
                    Err(_) => return None,
                };
                let client = CloudNodeServiceClient::new(
                    tarpc::client::Config::default(),
                    transport,
                )
                .spawn();

                match client
                    .query_received_images(context::current(), username)
                    .await {
                    Ok(result) => Some(result),
                    Err(_) => None,
                }
            });

            handles.push(handle);
        }

        let mut all_images = Vec::new();
        let mut found_any = false;

        for handle in handles {
            if let Ok(Some(Ok(images))) = handle.await {
                all_images.extend(images);
                found_any = true;
            }
        }

        if !found_any {
            return Err("Failed to connect to any cloud node".to_string());
        }

        // Remove duplicates
        all_images.sort_by(|a, b| a.image_id.cmp(&b.image_id));
        all_images.dedup_by(|a, b| a.image_id == b.image_id);

        info!(
            "[RpcClient {}] Total unique images found: {}",
            self.id,
            all_images.len()
        );
        Ok(all_images)
    }

    /// View an image
    pub async fn view_image(
        &self,
        username: String,
        image_id: String,
    ) -> Result<(Vec<u8>, u32), String> {
        info!(
            "[RpcClient {}] Viewing image {} for: {} (trying all nodes)",
            self.id, image_id, username
        );

        for address in &self.cloud_addresses {
            match self.connect_to_node(address).await {
                Ok(client) => {
                    match client
                        .view_image(context::current(), username.clone(), image_id.clone())
                        .await
                    {
                        Ok(Ok(result)) => {
                            info!(
                                "[RpcClient {}] Retrieved image from {}",
                                self.id, address
                            );
                            return Ok((result.image_data, result.remaining_views));
                        }
                        Ok(Err(e)) => {
                            // Image not on this node, try next
                            warn!("[RpcClient {}] Image not on {}: {}", self.id, address, e);
                            continue;
                        }
                        Err(e) => {
                            warn!("[RpcClient {}] RPC error: {}", self.id, e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    warn!("[RpcClient {}] Connection error: {}", self.id, e);
                    continue;
                }
            }
        }

        Err("Image not found on any node".to_string())
    }
}
