use crate::chunking::{ChunkedMessage, ChunkReassembler};
use crate::firebase::FireBaseClient;
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

    /// Create a new client by fetching node addresses from Firebase
    pub async fn from_firebase(id: usize) -> Result<Self, String> {
        let firebase = FireBaseClient::new();
        
        match firebase.get_all_nodes().await {
            Ok(nodes) => {
                let addresses: Vec<String> = nodes
                    .into_iter()
                    .map(|(_, node_info)| node_info.address)
                    .collect();

                if addresses.is_empty() {
                    Err("No nodes found in Firebase".to_string())
                } else {
                    info!("[Client {}] Found {} nodes from Firebase", id, addresses.len());
                    Ok(Self {
                        id,
                        cloud_addresses: addresses,
                    })
                }
            }
            Err(e) => Err(format!("Failed to fetch nodes from Firebase: {}", e)),
        }
    }

    /// Refresh the node list from Firebase
    pub async fn refresh_nodes(&mut self) -> Result<(), String> {
        let firebase = FireBaseClient::new();
        
        match firebase.get_all_nodes().await {
            Ok(nodes) => {
                let addresses: Vec<String> = nodes
                    .into_iter()
                    .map(|(_, node_info)| node_info.address)
                    .collect();

                if addresses.is_empty() {
                    Err("No nodes found in Firebase".to_string())
                } else {
                    info!("[Client {}] Refreshed node list: {} nodes", self.id, addresses.len());
                    self.cloud_addresses = addresses;
                    Ok(())
                }
            }
            Err(e) => Err(format!("Failed to refresh nodes from Firebase: {}", e)),
        }
    }

    /// Send message to nodes sequentially with retry logic
    /// Tries each node in order, if all fail, retries once, then returns maintenance error
    pub async fn send_with_retry(&self, message: Message) -> Result<Message, String> {
        let max_retries = 2; // Try twice (original + 1 retry)
        
        for retry in 0..max_retries {
            if retry > 0 {
                info!("[Client {}] Retrying all nodes (attempt {}/{})", self.id, retry + 1, max_retries);
                sleep(Duration::from_millis(500)).await; // Brief pause before retry
            }
            
            // Try each node sequentially, with 2 attempts per node
            for (i, address) in self.cloud_addresses.iter().enumerate() {
                for attempt in 1..=2 {
                    debug!("[Client {}] Trying node {} at {} (attempt {})", self.id, i + 1, address, attempt);
                    
                    match Self::send_to_node(self.id, address, message.clone()).await {
                        Ok(response) => {
                            debug!("[Client {}] Got response from node {} on attempt {}", self.id, i + 1, attempt);
                            return Ok(response);
                        }
                        Err(e) => {
                            warn!("[Client {}] Node {} ({}) failed on attempt {}: {}", self.id, i + 1, address, attempt, e);
                            if attempt == 1 {
                                sleep(Duration::from_millis(100)).await; // Brief pause before retry
                            }
                        }
                    }
                }
            }
        }
        
        // All nodes failed after retries
        error!("[Client {}] All nodes unreachable after {} attempts", self.id, max_retries);
        Err("SYSTEM_MAINTENANCE".to_string())
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
        info!("[Client {}] Trying {} cloud nodes...", self.id, self.cloud_addresses.len());

        // Try to register with any available node, 2 attempts per node
        let mut last_error = String::new();
        for (i, address) in self.cloud_addresses.iter().enumerate() {
            for attempt in 1..=2 {
                info!("[Client {}] Attempting connection to node {}/{}: {} (attempt {})", self.id, i + 1, self.cloud_addresses.len(), address, attempt);

                match Self::send_to_node(self.id, address, message.clone()).await {
                    Ok(Message::SessionRegisterResponse { success, error }) => {
                        if success {
                            info!("[Client {}] Successfully registered username: {}", self.id, username);
                            return Ok(());
                        } else {
                            let err_msg = error.unwrap_or_else(|| "Registration failed".to_string());
                            error!("[Client {}] Node {} rejected registration: {}", self.id, address, err_msg);
                            return Err(err_msg);
                        }
                    }
                    Ok(_) => {
                        let err_msg = "Unexpected response from server".to_string();
                        error!("[Client {}] Node {} sent unexpected response", self.id, address);
                        return Err(err_msg);
                    }
                    Err(e) => {
                        warn!("[Client {}] Failed to connect to node {} ({}): {}", self.id, i + 1, address, e);
                        last_error = format!("{}: {}", address, e);
                        continue;
                    }
                }
            }
        }

        error!("[Client {}] Failed to connect to any cloud node. Last error: {}", self.id, last_error);
        Err(format!("Failed to connect to any cloud node. Last error: {}", last_error))
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
        quota: u8,
    ) -> Result<Message, String> {
        let message = Message::EncryptionRequest {
            request_id: request_id.clone(),
            client_username,
            image_data,
            usernames,
            quota,
            forwarded: false,
            client_address: None, // Will be captured by first node that receives it
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

    /// Send message to a node without waiting for response (fire and forget)
    async fn send_to_node_no_response(
        client_id: usize,
        address: &str,
        message: Message,
    ) -> Result<(), String> {
        debug!("[Client {}] send_to_node_no_response: Sending to {}", client_id, address);

        // Create UDP socket
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(e) => {
                return Err(format!("Socket creation failed: {}", e));
            }
        };

        // Serialize and send message
        let message_bytes = serde_json::to_vec(&message).map_err(|e| e.to_string())?;
        let chunks = ChunkedMessage::fragment(message_bytes);

        for chunk in chunks {
            let chunk_bytes = serde_json::to_vec(&chunk).map_err(|e| e.to_string())?;
            socket.send_to(&chunk_bytes, address).await.map_err(|e| format!("Send error: {}", e))?;
        }

        debug!("[Client {}] Fire-and-forget message sent to {}", client_id, address);
        Ok(())
    }

    /// Send message to a specific node
    async fn send_to_node(
        client_id: usize,
        address: &str,
        message: Message,
    ) -> Result<Message, String> {
        debug!("[Client {}] send_to_node: Connecting to {}", client_id, address);

        // Create UDP socket
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => {
                debug!("[Client {}] Socket created successfully", client_id);
                s
            }
            Err(e) => {
                error!("[Client {}] Failed to create socket: {}", client_id, e);
                return Err(format!("Socket creation failed: {}", e));
            }
        };

        // Serialize message
        let message_bytes = serde_json::to_vec(&message).map_err(|e| {
            error!("[Client {}] Message serialization failed: {}", client_id, e);
            e.to_string()
        })?;

        debug!("[Client {}] Message serialized: {} bytes", client_id, message_bytes.len());

        // Use chunking for any size message
        let chunks = ChunkedMessage::fragment(message_bytes.clone());

        if chunks.len() == 1 {
            // Single packet - send directly
            let chunk_bytes = serde_json::to_vec(&chunks[0]).map_err(|e| {
                error!("[Client {}] Chunk serialization failed: {}", client_id, e);
                e.to_string()
            })?;

            debug!("[Client {}] Sending single packet: {} bytes to {}", client_id, chunk_bytes.len(), address);
            socket
                .send_to(&chunk_bytes, address)
                .await
                .map_err(|e| {
                    error!("[Client {}] Send failed to {}: {}", client_id, address, e);
                    format!("Send error: {}", e)
                })?;
        } else {
            // Multiple chunks - send with delay to prevent packet loss
            info!("[Client {}] Sending {} chunks ({} bytes total) to {}",
                  client_id, chunks.len(), message_bytes.len(), address);

            for (i, chunk) in chunks.iter().enumerate() {
                let chunk_bytes = serde_json::to_vec(&chunk).map_err(|e| {
                    error!("[Client {}] Chunk {} serialization failed: {}", client_id, i, e);
                    e.to_string()
                })?;

                socket
                    .send_to(&chunk_bytes, address)
                    .await
                    .map_err(|e| {
                        error!("[Client {}] Chunk {} send failed to {}: {}", client_id, i, address, e);
                        format!("Send error: {}", e)
                    })?;

                // Small delay between chunks to prevent UDP packet loss
                if i < chunks.len() - 1 {
                    sleep(Duration::from_millis(2)).await;
                }
            }

            info!("[Client {}] Successfully sent all {} chunks to {}", client_id, chunks.len(), address);
        }

        debug!("[Client {}] Successfully sent message to {}", client_id, address);

        // Create chunk reassembler for receiving response
        let mut reassembler = ChunkReassembler::new();
        let mut buffer = vec![0u8; 65535]; // Max UDP packet size

        // Loop to receive all chunks
        debug!("[Client {}] Waiting for response from {} (10s timeout)...", client_id, address);
        loop {
            // Read response with timeout
            let n = match tokio::time::timeout(Duration::from_secs(10), socket.recv_from(&mut buffer)).await
            {
                Ok(Ok((n, _))) => {
                    debug!("[Client {}] Received {} bytes from {}", client_id, n, address);
                    n
                }
                Ok(Err(e)) => {
                    error!("[Client {}] Receive error from {}: {}", client_id, address, e);
                    return Err(format!("Receive error: {}", e));
                }
                Err(_) => {
                    error!("[Client {}] Timeout waiting for response from {} (waited 10s)", client_id, address);
                    return Err(format!("Timeout waiting for response from {}", address));
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

    /// Login via node (node handles Firebase) - with retry logic
    pub async fn client_login(
        &self,
        user_id: String,
        password: String,
        client_ip: String,
    ) -> Result<Message, String> {
        let message = Message::ClientLogin {
            user_id: user_id.clone(),
            password,
            client_ip,
        };

        info!("[Client {}] Attempting login for user_id: {}", self.id, user_id);
        
        self.send_with_retry(message).await
    }

    /// Send heartbeat via node (node updates Firebase) - fire and forget
    pub async fn client_heartbeat(&self, user_id: String) {
        let message = Message::ClientHeartbeat { user_id };

        // Fire and forget to first available node (no response expected)
        for address in &self.cloud_addresses {
            if Self::send_to_node_no_response(self.id, address, message.clone()).await.is_ok() {
                return;
            }
        }
    }

    /// Logout via node (node updates Firebase) - fire and forget
    pub async fn client_logout(&self, user_id: String) {
        let message = Message::ClientLogout { user_id };

        // Fire and forget to first available node (no response expected)
        for address in &self.cloud_addresses {
            if Self::send_to_node_no_response(self.id, address, message.clone()).await.is_ok() {
                return;
            }
        }
    }

    /// Get user list via node
    pub async fn get_user_list(&self) -> Result<Vec<crate::messages::ClientUserInfo>, String> {
        let message = Message::GetUserList;

        for address in &self.cloud_addresses {
            for attempt in 1..=2 {
                match Self::send_to_node(self.id, address, message.clone()).await {
                    Ok(Message::GetUserListResponse { users }) => {
                        return Ok(users);
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        warn!("[Client {}] Failed to get user list via {} (attempt {}): {}", self.id, address, attempt, e);
                        if attempt == 1 {
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Get specific user info via node
    pub async fn get_user_info(&self, user_id: String) -> Result<Option<crate::messages::ClientUserInfo>, String> {
        let message = Message::GetUserInfo { user_id };

        for address in &self.cloud_addresses {
            for attempt in 1..=2 {
                match Self::send_to_node(self.id, address, message.clone()).await {
                    Ok(Message::GetUserInfoResponse { success, user_info, .. }) => {
                        if success {
                            return Ok(user_info);
                        } else {
                            return Ok(None);
                        }
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        warn!("[Client {}] Failed to get user info via {} (attempt {}): {}", self.id, address, attempt, e);
                        if attempt == 1 {
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Get user gallery via node
    pub async fn get_user_gallery(&self, user_id: String) -> Result<Vec<String>, String> {
        let message = Message::GetUserGallery { user_id };

        for address in &self.cloud_addresses {
            for attempt in 1..=2 {
                match Self::send_to_node(self.id, address, message.clone()).await {
                    Ok(Message::GetUserGalleryResponse { success, images, error }) => {
                        if success {
                            return Ok(images);
                        } else {
                            return Err(error.unwrap_or_else(|| "Failed to get gallery".to_string()));
                        }
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        warn!("[Client {}] Failed to get gallery via {} (attempt {}): {}", self.id, address, attempt, e);
                        if attempt == 1 {
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            }
        }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Check if a username is available (not already registered)
    pub async fn check_username_available(&self, username: String) -> Result<bool, String> {
        let message = Message::CheckUsernameAvailable {
            username: username.clone(),
        };

        // Try to check with any available node, 2 attempts per node
        for address in &self.cloud_addresses {
            for attempt in 1..=2 {
                match Self::send_to_node(self.id, address, message.clone()).await {
                    Ok(Message::CheckUsernameAvailableResponse { is_available, .. }) => {
                        return Ok(is_available);
                    }
                    Ok(_) => {
                        return Err("Unexpected response from server".to_string());
                    }
                    Err(e) => {
                        warn!("[Client {}] Failed to check with {} (attempt {}): {}", self.id, address, attempt, e);
                        if attempt == 1 {
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            }
            }

        Err("Failed to connect to any cloud node".to_string())
    }

    /// Send an encrypted image to other users
    /// Sends to ALL nodes for replication
    pub async fn send_image(
        &self,
        from_username: String,
        to_usernames: Vec<String>,
        encrypted_image: Vec<u8>,
        max_views: u8,
        image_id: String,
    ) -> Result<String, String> {
        let message = Message::SendImage {
            from_username: from_username.clone(),
            to_usernames: to_usernames.clone(),
            encrypted_image,
            max_views,
            image_id: image_id.clone(),
        };

        info!("[Client {}] Sending image {} to {:?} (replicating to {} nodes)",
              self.id, image_id, to_usernames, self.cloud_addresses.len());

        // Send to ALL nodes for replication (so queries can find the image on any node)
        let mut success_count = 0;
        let mut last_error = String::new();

        for address in &self.cloud_addresses {
            match Self::send_to_node(self.id, address, message.clone()).await {
                Ok(Message::SendImageResponse { success, image_id: _, error }) => {
                    if success {
                        success_count += 1;
                        debug!("[Client {}] Image replicated to {}", self.id, address);
                    } else {
                        last_error = error.unwrap_or_else(|| "Send failed".to_string());
                        warn!("[Client {}] Node {} rejected image: {}", self.id, address, last_error);
                    }
                }
                Ok(_) => {
                    last_error = "Unexpected response from server".to_string();
                    warn!("[Client {}] Unexpected response from {}", self.id, address);
                }
                Err(e) => {
                    last_error = e.clone();
                    warn!("[Client {}] Failed to send to {}: {}", self.id, address, e);
                }
            }
        }

        if success_count > 0 {
            info!("[Client {}] Successfully sent image {} to {}/{} nodes",
                  self.id, image_id, success_count, self.cloud_addresses.len());
            Ok(image_id)
        } else {
            Err(format!("Failed to send to any node. Last error: {}", last_error))
        }
    }

    /// Send an image request to a user via nodes (replicated to all nodes)
    pub async fn send_image_request(
        &self,
        request_id: String,
        from_id: u8,
        to_id: u8,
        from_username: String,
        to_username: String,
        image_index: usize,
        timestamp: i64,
    ) -> Result<String, String> {
        let message = Message::RequestImage {
            request_id: request_id.clone(),
            from_user_id: from_id.to_string(),
            from_username: from_username.clone(),
            to_user_id: to_id.to_string(),
            to_username: to_username.clone(),
            image_index,
            timestamp,
        };

        info!("[Client {}] Sending image request {} from {} to {} for image #{} (replicating to {} nodes)",
              self.id, request_id, from_username, to_username, image_index, self.cloud_addresses.len());

        let mut success_count = 0;
        let mut last_error = String::new();

        for address in &self.cloud_addresses {
            match Self::send_to_node(self.id, address, message.clone()).await {
                Ok(Message::RequestImageResponse { success, request_id: _, error }) => {
                    if success {
                        success_count += 1;
                        debug!("[Client {}] Image request replicated to {}", self.id, address);
                    } else {
                        last_error = error.unwrap_or_else(|| "Request failed".to_string());
                        warn!("[Client {}] Node {} rejected image request: {}", self.id, address, last_error);
                    }
                }
                Ok(_) => {
                    last_error = "Unexpected response from server".to_string();
                    warn!("[Client {}] Unexpected response from {}", self.id, address);
                }
                Err(e) => {
                    last_error = e.clone();
                    warn!("[Client {}] Failed to send to {}: {}", self.id, address, e);
                }
            }
        }

        if success_count > 0 {
            info!("[Client {}] Successfully sent image request {} to {}/{} nodes", self.id, request_id, success_count, self.cloud_addresses.len());
            Ok(request_id)
        } else {
            Err(format!("Failed to send to any node. Last error: {}", last_error))
        }
    }

    /// Send a text note to a user via nodes (replicated to all nodes)
    pub async fn send_note(
        &self,
        note_id: String,
        from_id: u8,
        to_id: u8,
        from_username: String,
        content: String,
        timestamp: i64,
    ) -> Result<String, String> {
        let message = Message::SendNote {
            note_id: note_id.clone(),
            from_id,
            to_id,
            from_username: from_username.clone(),
            content: content.clone(),
            timestamp,
        };

        info!("[Client {}] Sending note {} from {} to {} (replicating to {} nodes)",
              self.id, note_id, from_username, to_id, self.cloud_addresses.len());

        let mut success_count = 0;
        let mut last_error = String::new();

        for address in &self.cloud_addresses {
            match Self::send_to_node(self.id, address, message.clone()).await {
                Ok(Message::SendNoteResponse { success, note_id: _, error }) => {
                    if success {
                        success_count += 1;
                        debug!("[Client {}] Note replicated to {}", self.id, address);
                    } else {
                        last_error = error.unwrap_or_else(|| "Send failed".to_string());
                        warn!("[Client {}] Node {} rejected note: {}", self.id, address, last_error);
                    }
                }
                Ok(_) => {
                    last_error = "Unexpected response from server".to_string();
                    warn!("[Client {}] Unexpected response from {}", self.id, address);
                }
                Err(e) => {
                    last_error = e.clone();
                    warn!("[Client {}] Failed to send to {}: {}", self.id, address, e);
                }
            }
        }

        if success_count > 0 {
            info!("[Client {}] Successfully sent note {} to {}/{} nodes", self.id, note_id, success_count, self.cloud_addresses.len());
            Ok(to_id.to_string())
        } else {
            Err(format!("Failed to send to any node. Last error: {}", last_error))
        }
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

        // Query first available node (simple centralized approach)
        for address in &self.cloud_addresses {
            match Self::send_to_node(self.id, address, message.clone()).await {
                Ok(Message::QueryReceivedImagesResponse { images }) => {
                    info!("[Client {}] Found {} images", self.id, images.len());
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
    /// Tries nodes to find the image
    pub async fn view_image(
        &self,
        username: String,
        image_id: String,
    ) -> Result<(Vec<u8>, u8), String> {
        let message = Message::ViewImage {
            username: username.clone(),
            image_id: image_id.clone(),
        };

        info!("[Client {}] Viewing image {} for: {}", self.id, image_id, username);

        // Try nodes until one succeeds
        let mut last_error = String::new();

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

                        info!("[Client {}] Received encrypted image {} ({} bytes) from {}",
                              self.id, image_id, encrypted_data.len(), address);
                        eprintln!("[DEBUG] Starting decryption for image {} ({} bytes)", image_id, encrypted_data.len());

                        // Decrypt the image to extract metadata and get viewable image
                        match encryption::decrypt_image(encrypted_data.clone()).await {
                            Ok((decrypted_image, metadata)) => {
                                info!(
                                    "[Client {}] Successfully decrypted image {} - authorized users: {:?}, original quota: {}",
                                    self.id, image_id, metadata.usernames, metadata.quota
                                );
                                eprintln!("[DEBUG] Decryption successful! Image size: {} bytes", decrypted_image.len());
                                // Return the decrypted image (original image extracted from LSB steganography)
                                return Ok((decrypted_image, remaining));
                            }
                            Err(e) => {
                                error!("[Client {}] Failed to decrypt image {}: {}", self.id, image_id, e);
                                eprintln!("[DEBUG] Decryption failed: {}", e);
                                return Err(format!("Decryption failed: {}", e));
                            }
                        }
                    } else {
                        // Image not found on this node, try next
                        last_error = error.unwrap_or_else(|| "Image not found on this node".to_string());
                        warn!("[Client {}] Image {} not on {}: {}", self.id, image_id, address, last_error);
                        continue;
                    }
                }
                Ok(_) => {
                    last_error = "Unexpected response from server".to_string();
                    warn!("[Client {}] Unexpected response from {}", self.id, address);
                    continue;
                }
                Err(e) => {
                    last_error = format!("Connection failed: {}", e);
                    warn!("[Client {}] Failed to view from {}: {}", self.id, address, e);
                    continue;
                }
            }
        }

        Err(format!("Image not found on any node. Last error: {}", last_error))
    }

    /// Download an image in encrypted form (for local caching)
    /// Returns the encrypted data without decrypting it
    pub async fn download_image_encrypted(
        &self,
        username: String,
        image_id: String,
    ) -> Result<Vec<u8>, String> {
        let message = Message::ViewImage {
            username: username.clone(),
            image_id: image_id.clone(),
        };

        info!("[Client {}] Downloading encrypted image {} for: {}", self.id, image_id, username);

        // Try nodes until one succeeds
        let mut last_error = String::new();

        for address in &self.cloud_addresses {
            match Self::send_to_node(self.id, address, message.clone()).await {
                Ok(Message::ViewImageResponse {
                    success,
                    image_data,
                    remaining_views: _,
                    error,
                }) => {
                    if success {
                        let encrypted_data = image_data.ok_or_else(|| "No image data returned".to_string())?;
                        info!("[Client {}] Downloaded encrypted image {} ({} bytes)",
                              self.id, image_id, encrypted_data.len());
                        return Ok(encrypted_data);
                    } else {
                        last_error = error.unwrap_or_else(|| "Image not found on this node".to_string());
                        warn!("[Client {}] Image {} not on {}: {}", self.id, image_id, address, last_error);
                        continue;
                    }
                }
                Ok(_) => {
                    last_error = "Unexpected response from server".to_string();
                    continue;
                }
                Err(e) => {
                    last_error = format!("Connection failed: {}", e);
                    continue;
                }
            }
        }

        Err(format!("Image not found on any node. Last error: {}", last_error))
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
