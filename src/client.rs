use crate::chunking::{ChunkedMessage, ChunkReassembler};
use crate::messages::Message;
use crate::metrics::MetricsCollector;
use crate::encryption;
use log::{debug, error, info, warn};
use rand::Rng;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::{sleep, Instant};

/// Client that sends encryption requests to the cloud
pub struct Client {
    pub id: usize,
    pub cloud_addresses: Vec<String>,
}

impl Client {
    pub fn new(id: usize, cloud_addresses: Vec<String>) -> Self {
        Self {
            id,
            cloud_addresses,
        }
    }

    /// Register a session with a username
    /// Returns Ok(()) if successful, Err with error message if username is taken
    pub async fn register_session(
        &self,
        client_id: String,
        username: String,
    ) -> Result<(), String> {
        let message = Message::SessionRegister {
            client_id: client_id.clone(),
            username: username.clone(),
        };

        info!("[Client {}] Registering username: {}", self.id, username);

        // Try to register with any available node
        for address in &self.cloud_addresses {
            match Self::send_to_node(self.id, address, message.clone()).await {
                Ok(Message::SessionRegisterResponse { success, error }) => {
                    if success {
                        info!("[Client {}] Successfully registered username: {}", self.id, username);
                        return Ok(());
                    } else {
                        return Err(error.unwrap_or_else(|| "Registration failed".to_string()));
                    }
                }
                Ok(_) => {
                    return Err("Unexpected response from server".to_string());
                }
                Err(e) => {
                    warn!("[Client {}] Failed to register with {}: {}", self.id, address, e);
                    continue;
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Unregister a session
    pub async fn unregister_session(&self, client_id: String, username: String) {
        let message = Message::SessionUnregister {
            client_id,
            username: username.clone(),
        };

        info!("[Client {}] Unregistering username: {}", self.id, username);

        // Send to all nodes (fire and forget)
        for address in &self.cloud_addresses {
            let address = address.clone();
            let message = message.clone();
            let id = self.id;
            tokio::spawn(async move {
                let _ = Self::send_to_node(id, &address, message).await;
            });
        }
    }

    /// Send an encryption request by multicasting to all cloud nodes
    /// Returns the first successful response
    pub async fn send_encryption_request(
        &self,
        request_id: String,
        client_username: String,
        image_data: Vec<u8>,
        usernames: Vec<String>,
        quota: u32,
    ) -> Result<Message, String> {
        let message = Message::EncryptionRequest {
            request_id: request_id.clone(),
            client_username,
            image_data,
            usernames,
            quota,
            forwarded: false,
        };

        debug!("[Client {}] Multicasting request: {}", self.id, request_id);

        // Multicast to all cloud nodes
        let mut handles = vec![];

        for address in &self.cloud_addresses {
            let address = address.clone();
            let message = message.clone();
            let client_id = self.id;

            let handle = tokio::spawn(async move {
                Self::send_to_node(client_id, &address, message).await
            });

            handles.push(handle);
        }

        // Wait for first successful response
        for handle in handles {
            if let Ok(Ok(response)) = handle.await {
                return Ok(response);
            }
        }

        Err("All nodes failed to respond".to_string())
    }

    /// Send message to a specific node
    async fn send_to_node(
        client_id: usize,
        address: &str,
        message: Message,
    ) -> Result<Message, String> {
        // Create UDP socket
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(e) => {
                warn!("[Client {}] Failed to create socket: {}", client_id, e);
                return Err(format!("Socket creation failed: {}", e));
            }
        };

        // Serialize message
        let message_bytes = serde_json::to_vec(&message).map_err(|e| e.to_string())?;

        // Check message size
        if message_bytes.len() > 65507 {
            return Err("Message exceeds UDP packet size limit".to_string());
        }

        // Send message
        socket
            .send_to(&message_bytes, address)
            .await
            .map_err(|e| format!("Send error: {}", e))?;

        debug!("[Client {}] Sent {} bytes to {}", client_id, message_bytes.len(), address);

        // Create chunk reassembler for receiving response
        let mut reassembler = ChunkReassembler::new();
        let mut buffer = vec![0u8; 65535]; // Max UDP packet size

        // Loop to receive all chunks
        loop {
            // Read response with timeout
            let n = match tokio::time::timeout(Duration::from_secs(10), socket.recv_from(&mut buffer)).await
            {
                Ok(Ok((n, _))) => n,
                Ok(Err(e)) => {
                    return Err(format!("Receive error: {}", e));
                }
                Err(_) => {
                    return Err("Timeout waiting for response".to_string());
                }
            };

            if n == 0 {
                return Err("Empty response".to_string());
            }

            // Try to parse as ChunkedMessage first
            match serde_json::from_slice::<ChunkedMessage>(&buffer[..n]) {
                Ok(chunked_message) => {
                    debug!("[Client {}] Received chunk from {}", client_id, address);

                    // Process chunk through reassembler
                    if let Some(complete_data) = reassembler.process_chunk(chunked_message) {
                        debug!("[Client {}] All chunks received, reassembled {} bytes", client_id, complete_data.len());

                        // Parse complete message
                        let response: Message = serde_json::from_slice(&complete_data)
                            .map_err(|e| format!("Failed to parse reassembled message: {}", e))?;

                        debug!("[Client {}] Received response from {}", client_id, address);
                        return Ok(response);
                    } else {
                        // Need more chunks, continue loop
                        debug!("[Client {}] Waiting for more chunks...", client_id);
                        continue;
                    }
                }
                Err(_) => {
                    // Not a chunked message, try parsing as direct Message (for small responses)
                    match serde_json::from_slice::<Message>(&buffer[..n]) {
                        Ok(response) => {
                            debug!("[Client {}] Received direct (non-chunked) response from {}", client_id, address);
                            return Ok(response);
                        }
                        Err(e) => {
                            return Err(format!("Failed to parse response: {}", e));
                        }
                    }
                }
            }
        }
    }

    /// Check if a username is available (not already registered)
    pub async fn check_username_available(&self, username: String) -> Result<bool, String> {
        let message = Message::CheckUsernameAvailable {
            username: username.clone(),
        };

        // Try to check with any available node
        for address in &self.cloud_addresses {
            match Self::send_to_node(self.id, address, message.clone()).await {
                Ok(Message::CheckUsernameAvailableResponse { is_available, .. }) => {
                    return Ok(is_available);
                }
                Ok(_) => {
                    return Err("Unexpected response from server".to_string());
                }
                Err(e) => {
                    warn!("[Client {}] Failed to check with {}: {}", self.id, address, e);
                    continue;
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Send an encrypted image to other users
    pub async fn send_image(
        &self,
        from_username: String,
        to_usernames: Vec<String>,
        encrypted_image: Vec<u8>,
        max_views: u32,
        image_id: String,
    ) -> Result<String, String> {
        let message = Message::SendImage {
            from_username: from_username.clone(),
            to_usernames: to_usernames.clone(),
            encrypted_image,
            max_views,
            image_id: image_id.clone(),
        };

        info!("[Client {}] Sending image {} to {:?}", self.id, image_id, to_usernames);

        // Try to send to any available node
        for address in &self.cloud_addresses {
            match Self::send_to_node(self.id, address, message.clone()).await {
                Ok(Message::SendImageResponse { success, image_id, error }) => {
                    if success {
                        info!("[Client {}] Successfully sent image: {}", self.id, image_id);
                        return Ok(image_id);
                    } else {
                        return Err(error.unwrap_or_else(|| "Send failed".to_string()));
                    }
                }
                Ok(_) => {
                    return Err("Unexpected response from server".to_string());
                }
                Err(e) => {
                    warn!("[Client {}] Failed to send to {}: {}", self.id, address, e);
                    continue;
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Query received images for a username
    pub async fn query_received_images(
        &self,
        username: String,
    ) -> Result<Vec<crate::messages::ReceivedImageInfo>, String> {
        let message = Message::QueryReceivedImages {
            username: username.clone(),
        };

        info!("[Client {}] Querying received images for: {}", self.id, username);

        // Try to query from any available node
        for address in &self.cloud_addresses {
            match Self::send_to_node(self.id, address, message.clone()).await {
                Ok(Message::QueryReceivedImagesResponse { images }) => {
                    info!("[Client {}] Found {} images for {}", self.id, images.len(), username);
                    return Ok(images);
                }
                Ok(_) => {
                    return Err("Unexpected response from server".to_string());
                }
                Err(e) => {
                    warn!("[Client {}] Failed to query {}: {}", self.id, address, e);
                    continue;
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// View an image (decrements the view counter)
    pub async fn view_image(
        &self,
        username: String,
        image_id: String,
    ) -> Result<(Vec<u8>, u32), String> {
        let message = Message::ViewImage {
            username: username.clone(),
            image_id: image_id.clone(),
        };

        info!("[Client {}] Viewing image {} for: {}", self.id, image_id, username);

        // Try to view from any available node
        for address in &self.cloud_addresses {
            match Self::send_to_node(self.id, address, message.clone()).await {
                Ok(Message::ViewImageResponse {
                    success,
                    image_data,
                    remaining_views,
                    error,
                }) => {
                    if success {
                        let encrypted_data = image_data.ok_or_else(|| "No image data returned".to_string())?;
                        let remaining = remaining_views.ok_or_else(|| "No view count returned".to_string())?;

                        info!("[Client {}] Received encrypted image {} ({} bytes) for viewing", self.id, image_id, encrypted_data.len());
                        eprintln!("[DEBUG] Starting decryption for image {} ({} bytes)", image_id, encrypted_data.len());

                        // Decrypt the image to extract metadata and get viewable image
                        match encryption::decrypt_image(encrypted_data.clone()).await {
                            Ok((decrypted_image, metadata)) => {
                                info!(
                                    "[Client {}] Successfully decrypted image {} - authorized users: {:?}, original quota: {}",
                                    self.id, image_id, metadata.usernames, metadata.quota
                                );
                                eprintln!("[DEBUG] Decryption successful! Image size: {} bytes", decrypted_image.len());
                                // Return the decrypted image (original unscrambled)
                                return Ok((decrypted_image, remaining));
                            }
                            Err(e) => {
                                error!("[Client {}] Failed to decrypt image {}: {}", self.id, image_id, e);
                                eprintln!("[DEBUG] Decryption failed: {}", e);
                                return Err(format!("Decryption failed: {}", e));
                            }
                        }
                    } else {
                        return Err(error.unwrap_or_else(|| "View failed".to_string()));
                    }
                }
                Ok(_) => {
                    return Err("Unexpected response from server".to_string());
                }
                Err(e) => {
                    warn!("[Client {}] Failed to view from {}: {}", self.id, address, e);
                    continue;
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Generate a random test image
    fn generate_test_image(size_kb: usize) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        (0..size_kb * 1024).map(|_| rng.gen()).collect()
    }

    /// Run a single test request
    pub async fn run_test_request(&self, request_num: usize) -> (bool, u64) {
        let start = Instant::now();

        let request_id = format!("client_{}_req_{}", self.id, request_num);
        let image_data = Self::generate_test_image(10); // 10KB image
        let client_username = format!("stress_test_user_{}", self.id);
        let usernames = vec![
            format!("user_{}", self.id),
            format!("user_{}", (self.id + 1) % 100),
        ];
        let quota = 5;

        match self
            .send_encryption_request(request_id.clone(), client_username, image_data, usernames, quota)
            .await
        {
            Ok(Message::EncryptionResponse { success, error, .. }) => {
                let duration = start.elapsed().as_millis() as u64;

                if success {
                    debug!(
                        "[Client {}] Request {} succeeded in {}ms",
                        self.id, request_id, duration
                    );
                    (true, duration)
                } else {
                    warn!(
                        "[Client {}] Request {} failed: {:?}",
                        self.id, request_id, error
                    );
                    (false, duration)
                }
            }
            Ok(_) => {
                warn!("[Client {}] Unexpected response for {}", self.id, request_id);
                (false, start.elapsed().as_millis() as u64)
            }
            Err(e) => {
                error!("[Client {}] Request {} error: {}", self.id, request_id, e);
                (false, start.elapsed().as_millis() as u64)
            }
        }
    }
}

/// Run stress test with multiple concurrent clients
pub async fn run_stress_test(
    num_clients: usize,
    requests_per_client: usize,
    cloud_addresses: Vec<String>,
    metrics: MetricsCollector,
) {
    info!(
        "Starting stress test: {} clients x {} requests = {} total requests",
        num_clients,
        requests_per_client,
        num_clients * requests_per_client
    );

    let mut handles = vec![];

    for client_id in 0..num_clients {
        let cloud_addresses = cloud_addresses.clone();
        let metrics = metrics.clone();

        let handle = tokio::spawn(async move {
            let client = Client::new(client_id, cloud_addresses);

            for req_num in 0..requests_per_client {
                let (success, duration) = client.run_test_request(req_num).await;

                // Record metrics
                {
                    let mut m = metrics.lock().await;
                    m.record_request(success, duration);
                }

                // Small delay between requests to simulate realistic behavior
                if req_num < requests_per_client - 1 {
                    sleep(Duration::from_millis(10)).await;
                }
            }

            info!("[Client {}] Completed all {} requests", client_id, requests_per_client);
        });

        handles.push(handle);

        // Stagger client start times slightly
        if client_id < num_clients - 1 {
            sleep(Duration::from_millis(5)).await;
        }
    }

    // Wait for all clients to complete
    for handle in handles {
        let _ = handle.await;
    }

    // Mark test as finished
    {
        let mut m = metrics.lock().await;
        m.finish();
    }

    info!("Stress test completed!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_test_image() {
        let image = Client::generate_test_image(1);
        assert_eq!(image.len(), 1024);
    }

    #[test]
    fn test_client_creation() {
        let addresses = vec!["127.0.0.1:8001".to_string()];
        let client = Client::new(1, addresses.clone());
        assert_eq!(client.id, 1);
        assert_eq!(client.cloud_addresses, addresses);
    }
}
