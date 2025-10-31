use crate::election::{ElectionManager, ElectionResult};
use crate::encryption;
use crate::messages::{Message, NodeId, NodeState, ReceivedImageInfo};
use log::{debug, error, info, warn};
use rand::Rng;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, sleep};

/// Stored image data
#[derive(Clone, Debug)]
pub struct StoredImage {
    pub image_id: String,
    pub from_username: String,
    pub encrypted_data: Vec<u8>,
    pub remaining_views: u32,
    pub max_views: u32,
    pub timestamp: i64,
}

/// Cloud Node that participates in the distributed system
pub struct CloudNode {
    pub id: NodeId,
    pub address: String,
    pub state: Arc<RwLock<NodeState>>,
    pub election_manager: Arc<Mutex<ElectionManager>>,
    pub current_load: Arc<RwLock<f64>>,
    pub queue_length: Arc<RwLock<usize>>,
    pub peer_addresses: HashMap<NodeId, String>,
    pub processed_requests: Arc<RwLock<usize>>,
    pub active_sessions: Arc<RwLock<HashMap<String, String>>>, // username -> client_id
    pub stored_images: Arc<RwLock<HashMap<String, Vec<StoredImage>>>>, // username -> list of images
}

impl CloudNode {
    pub fn new(id: NodeId, address: String, peer_addresses: HashMap<NodeId, String>) -> Self {
        let election_manager = ElectionManager::new(id, peer_addresses.clone());

        Self {
            id,
            address: address.clone(),
            state: Arc::new(RwLock::new(NodeState::Active)),
            election_manager: Arc::new(Mutex::new(election_manager)),
            current_load: Arc::new(RwLock::new(0.0)),
            queue_length: Arc::new(RwLock::new(0)),
            peer_addresses,
            processed_requests: Arc::new(RwLock::new(0)),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            stored_images: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the cloud node server
    pub async fn start(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        info!("[Node {}] Starting on {}", self.id, self.address);

        let socket = UdpSocket::bind(&self.address).await?;
        info!("[Node {}] Listening on {} (UDP)", self.id, self.address);

        // Start background tasks
        // PRODUCTION MODE: Failure simulation disabled for controlled testing
        // let self_clone = self.clone();
        // tokio::spawn(async move {
        //     self_clone.failure_simulation_task().await;
        // });

        let self_clone = self.clone();
        tokio::spawn(async move {
            self_clone.periodic_election_task().await;
        });

        // Receive incoming datagrams
        let socket = Arc::new(socket);
        let mut buffer = vec![0u8; 65535]; // Max UDP packet size

        loop {
            match socket.recv_from(&mut buffer).await {
                Ok((n, addr)) => {
                    let data = buffer[..n].to_vec();
                    let self_clone = self.clone();
                    let socket_clone = socket.clone();

                    tokio::spawn(async move {
                        if let Err(e) = self_clone.handle_datagram(socket_clone, data, addr).await {
                            error!("[Node {}] Error handling datagram: {}", self_clone.id, e);
                        }
                    });
                }
                Err(e) => {
                    error!("[Node {}] Error receiving datagram: {}", self.id, e);
                }
            }
        }
    }

    /// Handle incoming UDP datagram
    async fn handle_datagram(
        &self,
        socket: Arc<UdpSocket>,
        data: Vec<u8>,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if node is in Failed state
        let state = self.state.read().await;
        if *state == NodeState::Failed {
            debug!("[Node {}] Ignoring datagram (FAILED state)", self.id);
            return Ok(());
        }
        drop(state);

        // Parse message
        let message: Message = serde_json::from_slice(&data)?;
        debug!("[Node {}] Received from {}: {}", self.id, addr, message);

        // Process message based on type
        let response = self.process_message(message).await;

        // Send response if any
        if let Some(response) = response {
            let response_bytes = serde_json::to_vec(&response)?;

            // Check if response fits in UDP packet
            if response_bytes.len() > 65507 {
                error!("[Node {}] Response too large for UDP: {} bytes", self.id, response_bytes.len());
                return Err("Response exceeds UDP packet size limit".into());
            }

            socket.send_to(&response_bytes, addr).await?;
            debug!("[Node {}] Sent response to {}", self.id, addr);
        }

        Ok(())
    }

    /// Process incoming message
    async fn process_message(&self, message: Message) -> Option<Message> {
        match message {
            Message::SessionRegister { client_id, username } => {
                let mut sessions = self.active_sessions.write().await;

                // Check if username is already taken
                if sessions.contains_key(&username) {
                    info!("[Node {}] Session registration failed: username '{}' already taken", self.id, username);
                    Some(Message::SessionRegisterResponse {
                        success: false,
                        error: Some(format!("Username '{}' is already in use", username)),
                    })
                } else {
                    // Register the session
                    sessions.insert(username.clone(), client_id.clone());
                    info!("[Node {}] Session registered: username '{}' for client '{}'", self.id, username, client_id);
                    Some(Message::SessionRegisterResponse {
                        success: true,
                        error: None,
                    })
                }
            }

            Message::SessionUnregister { client_id: _, username } => {
                let mut sessions = self.active_sessions.write().await;
                sessions.remove(&username);
                info!("[Node {}] Session unregistered: username '{}'", self.id, username);
                None
            }

            Message::EncryptionRequest {
                request_id,
                client_username,
                image_data,
                usernames,
                quota,
                forwarded,
            } => {
                if forwarded {
                    // Request forwarded by coordinator - MUST process locally regardless of current state
                    // This prevents loops and ensures coordinator's decision is respected
                    info!("[Node {}] Processing forwarded request {} locally (from coordinator)", self.id, request_id);
                    
                    // Increment queue length
                    {
                        let mut queue = self.queue_length.write().await;
                        *queue += 1;
                    }

                    // Process encryption
                    let self_clone = Arc::new(self.clone());
                    let result = self_clone
                        .process_encryption_request(request_id.clone(), image_data, usernames, quota)
                        .await;

                    // Decrement queue length
                    {
                        let mut queue = self.queue_length.write().await;
                        *queue = queue.saturating_sub(1);
                    }

                    Some(result)
                } else {
                    // Get current coordinator (may change due to elections)
                    let manager = self.election_manager.lock().await;
                    let coordinator_id = manager.get_coordinator().unwrap_or(self.id);
                    drop(manager); // Release lock immediately
                    
                    if coordinator_id != self.id {
                        // Not coordinator - forward to current coordinator for load balancing
                        info!("[Node {}] Forwarding request {} to coordinator Node {} for load balancing", 
                              self.id, request_id, coordinator_id);
                        
                        let forward_message = Message::EncryptionRequest {
                            request_id: request_id.clone(),
                            client_username,
                            image_data,
                            usernames,
                            quota,
                            forwarded: false, // Coordinator will do load balancing
                        };
                        
                        match self.send_message_to_node(coordinator_id, forward_message).await {
                            Ok(Some(response)) => {
                                info!("[Node {}] Received response from coordinator Node {} for {}", 
                                      self.id, coordinator_id, request_id);
                                Some(response)
                            }
                            Ok(None) => {
                                warn!("[Node {}] No response from coordinator Node {} for {}", 
                                      self.id, coordinator_id, request_id);
                                // Coordinator may have failed - try processing locally as fallback
                                info!("[Node {}] Coordinator unresponsive, processing {} locally as fallback", 
                                      self.id, request_id);
                                
                                {
                                    let mut queue = self.queue_length.write().await;
                                    *queue += 1;
                                }
                                
                                let self_clone = Arc::new(self.clone());
                                let result = self_clone
                                    .process_encryption_request(request_id.clone(), image_data, usernames, quota)
                                    .await;
                                
                                {
                                    let mut queue = self.queue_length.write().await;
                                    *queue = queue.saturating_sub(1);
                                }
                                
                                Some(result)
                            }
                            Err(e) => {
                                error!("[Node {}] Failed to contact coordinator Node {} for {}: {}", 
                                       self.id, coordinator_id, request_id, e);
                                // Process locally as fallback
                                info!("[Node {}] Processing {} locally due to coordinator error", 
                                      self.id, request_id);
                                
                                {
                                    let mut queue = self.queue_length.write().await;
                                    *queue += 1;
                                }
                                
                                let self_clone = Arc::new(self.clone());
                                let result = self_clone
                                    .process_encryption_request(request_id.clone(), image_data, usernames, quota)
                                    .await;
                                
                                {
                                    let mut queue = self.queue_length.write().await;
                                    *queue = queue.saturating_sub(1);
                                }
                                
                                Some(result)
                            }
                        }
                    } else {
                        // This node IS the coordinator - perform load balancing
                        info!("[Node {}] Coordinator performing load balancing for request {}", self.id, request_id);
                        
                        // Query all nodes for their current load
                        let lowest_load_node = self.find_lowest_load_node().await;
                        
                        info!("[Node {}] Load balancing: Selected Node {} for request {}", 
                              self.id, lowest_load_node, request_id);
                        
                        if lowest_load_node == self.id {
                            // This coordinator has lowest load - process locally
                            info!("[Node {}] Processing request {} locally (lowest load)", self.id, request_id);
                            
                            // Increment queue length
                            {
                                let mut queue = self.queue_length.write().await;
                                *queue += 1;
                            }

                            // Process encryption
                            let self_clone = Arc::new(self.clone());
                            let result = self_clone
                                .process_encryption_request(request_id.clone(), image_data, usernames, quota)
                                .await;

                            // Decrement queue length
                            {
                                let mut queue = self.queue_length.write().await;
                                *queue = queue.saturating_sub(1);
                            }

                            Some(result)
                        } else {
                            // Forward to lowest-load node
                            info!("[Node {}] Forwarding request {} to lowest-load Node {}", 
                                  self.id, request_id, lowest_load_node);
                            
                            let forward_message = Message::EncryptionRequest {
                                request_id: request_id.clone(),
                                client_username,
                                image_data,
                                usernames,
                                quota,
                                forwarded: true, // Mark as forwarded to prevent loops
                            };
                            
                            match self.send_message_to_node(lowest_load_node, forward_message).await {
                                Ok(Some(response)) => {
                                    info!("[Node {}] Received response from Node {} for {}", 
                                          self.id, lowest_load_node, request_id);
                                    Some(response)
                                }
                                Ok(None) => {
                                    warn!("[Node {}] No response from Node {} for {}", 
                                          self.id, lowest_load_node, request_id);
                                    Some(Message::EncryptionResponse {
                                        request_id,
                                        encrypted_image: vec![],
                                        success: false,
                                        error: Some("Selected node did not respond".to_string()),
                                    })
                                }
                                Err(e) => {
                                    error!("[Node {}] Failed to forward to Node {} for {}: {}", 
                                           self.id, lowest_load_node, request_id, e);
                                    Some(Message::EncryptionResponse {
                                        request_id,
                                        encrypted_image: vec![],
                                        success: false,
                                        error: Some(format!("Forward to selected node failed: {}", e)),
                                    })
                                }
                            }
                        }
                    }
                }
            }

            Message::Election { from_node } => {
                let load = *self.current_load.read().await;
                let manager = self.election_manager.lock().await;

                let send_fn = |node: NodeId, msg: Message| {
                    let self_clone = self.clone();
                    tokio::spawn(async move {
                        let _ = self_clone.send_message_to_node(node, msg).await;
                    });
                    true
                };

                manager.handle_election_message(from_node, load, send_fn);
                None
            }

            Message::LoadQuery { from_node: _ } => {
                let load = *self.current_load.read().await;
                let queue = *self.queue_length.read().await;
                let processed = *self.processed_requests.read().await;
                Some(Message::LoadResponse {
                    node_id: self.id,
                    load,
                    queue_length: queue,
                    processed_count: processed,
                })
            }

            Message::Coordinator { node_id, load } => {
                let mut manager = self.election_manager.lock().await;
                manager.update_coordinator(node_id, load);
                None
            }

            Message::StateSync { from_node: _ } => {
                let manager = self.election_manager.lock().await;
                let coordinator_id = manager.get_coordinator().unwrap_or(self.id);
                Some(Message::StateSyncResponse {
                    coordinator_id,
                    load_metrics: vec![],
                    timestamp: chrono::Utc::now().timestamp(),
                })
            }

            Message::CoordinatorQuery => {
                let manager = self.election_manager.lock().await;
                let coordinator_id = manager.get_coordinator().unwrap_or(self.id);
                
                // Map coordinator ID to address
                let coordinator_address = self.peer_addresses.get(&coordinator_id)
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|| self.address.to_string());
                
                Some(Message::CoordinatorQueryResponse {
                    coordinator_address,
                })
            }

            Message::SendImage {
                from_username,
                to_usernames,
                encrypted_image,
                max_views,
                image_id,
            } => {
                let mut stored = self.stored_images.write().await;
                let timestamp = chrono::Utc::now().timestamp();

                for username in to_usernames {
                    let image = StoredImage {
                        image_id: image_id.clone(),
                        from_username: from_username.clone(),
                        encrypted_data: encrypted_image.clone(),
                        remaining_views: max_views,
                        max_views,
                        timestamp,
                    };

                    stored.entry(username.clone()).or_insert_with(Vec::new).push(image);
                }

                info!("[Node {}] Stored image {} from {}", self.id, image_id, from_username);

                Some(Message::SendImageResponse {
                    success: true,
                    image_id,
                    error: None,
                })
            }

            Message::QueryReceivedImages { username } => {
                let stored = self.stored_images.read().await;
                let images = stored
                    .get(&username)
                    .map(|imgs| {
                        imgs.iter()
                            .filter(|img| img.remaining_views > 0)
                            .map(|img| ReceivedImageInfo {
                                image_id: img.image_id.clone(),
                                from_username: img.from_username.clone(),
                                remaining_views: img.remaining_views,
                                timestamp: img.timestamp,
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                Some(Message::QueryReceivedImagesResponse { images })
            }

            Message::CheckUsernameAvailable { username } => {
                let sessions = self.active_sessions.read().await;
                let is_available = !sessions.contains_key(&username);
                Some(Message::CheckUsernameAvailableResponse {
                    username,
                    is_available,
                })
            }

            Message::ViewImage { username, image_id } => {
                let mut stored = self.stored_images.write().await;

                if let Some(user_images) = stored.get_mut(&username) {
                    if let Some(img) = user_images.iter_mut().find(|i| i.image_id == image_id) {
                        if img.remaining_views > 0 {
                            img.remaining_views -= 1;
                            info!(
                                "[Node {}] User {} viewed image {} (remaining: {})",
                                self.id, username, image_id, img.remaining_views
                            );
                            Some(Message::ViewImageResponse {
                                success: true,
                                image_data: Some(img.encrypted_data.clone()),
                                remaining_views: Some(img.remaining_views),
                                error: None,
                            })
                        } else {
                            Some(Message::ViewImageResponse {
                                success: false,
                                image_data: None,
                                remaining_views: Some(0),
                                error: Some("No views remaining".to_string()),
                            })
                        }
                    } else {
                        Some(Message::ViewImageResponse {
                            success: false,
                            image_data: None,
                            remaining_views: None,
                            error: Some("Image not found".to_string()),
                        })
                    }
                } else {
                    Some(Message::ViewImageResponse {
                        success: false,
                        image_data: None,
                        remaining_views: None,
                        error: Some("No images for this user".to_string()),
                    })
                }
            }

            _ => None,
        }
    }

    /// Process encryption request
    async fn process_encryption_request(
        &self,
        request_id: String,
        image_data: Vec<u8>,
        usernames: Vec<String>,
        quota: u32,
    ) -> Message {
        info!(
            "[Node {}] Processing encryption request: {}",
            self.id, request_id
        );

        // Update load (based on queue length)
        {
            let queue = *self.queue_length.read().await;
            let mut load = self.current_load.write().await;
            // Load is queue length directly: reflects actual workload
            *load = queue as f64;
        }

        // Perform encryption
        match encryption::encrypt_image(image_data, usernames, quota).await {
            Ok(encrypted_image) => {
                let mut processed = self.processed_requests.write().await;
                *processed += 1;

                info!(
                    "[Node {}] Successfully encrypted request: {} (total: {})",
                    self.id, request_id, *processed
                );

                Message::EncryptionResponse {
                    request_id,
                    encrypted_image,
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!(
                    "[Node {}] Encryption failed for request {}: {}",
                    self.id, request_id, e
                );

                Message::EncryptionResponse {
                    request_id,
                    encrypted_image: vec![],
                    success: false,
                    error: Some(e),
                }
            }
        }
    }

    /// Find the node with the lowest load (including self)
    async fn find_lowest_load_node(&self) -> NodeId {
        let mut lowest_load = *self.current_load.read().await;
        let mut lowest_node = self.id;
        
        info!("[Node {}] Current load: {:.2}", self.id, lowest_load);
        
        // Query all peer nodes for their load SEQUENTIALLY (more stable)
        for (peer_id, _) in &self.peer_addresses {
            let load_query = Message::LoadQuery { from_node: self.id };
            
            match self.send_message_to_node(*peer_id, load_query).await {
                Ok(Some(Message::LoadResponse { node_id, load, queue_length, processed_count })) => {
                    info!("[Node {}] Node {} load: {:.2} (queue: {}, processed: {})", 
                          self.id, node_id, load, queue_length, processed_count);
                    
                    if load < lowest_load {
                        lowest_load = load;
                        lowest_node = node_id;
                    }
                }
                Ok(Some(other_msg)) => {
                    warn!("[Node {}] Unexpected response from Node {}: {:?}", 
                          self.id, peer_id, other_msg);
                }
                Ok(None) => {
                    warn!("[Node {}] No response from Node {} (timeout)", self.id, peer_id);
                }
                Err(e) => {
                    warn!("[Node {}] Failed to query load from Node {}: {}", self.id, peer_id, e);
                }
            }
        }
        
        info!("[Node {}] Lowest load node: {} (load: {:.2})", self.id, lowest_node, lowest_load);
        lowest_node
    }

    /// Send message to another node
    async fn send_message_to_node(&self, node_id: NodeId, message: Message) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        if let Some(address_str) = self.peer_addresses.get(&node_id) {
            // Parse the address string to SocketAddr
            let address: SocketAddr = address_str.parse()
                .map_err(|e| format!("Invalid address '{}': {}", address_str, e))?;
            
            // Create a temporary UDP socket bound to a specific port (node's port + 1000)
            // This avoids using random ephemeral ports that might be blocked
            let bind_addr = format!("0.0.0.0:{}", 9000 + self.id);
            let socket = match UdpSocket::bind(&bind_addr).await {
                Ok(s) => s,
                Err(_) => {
                    // Fallback to any available port if specific port fails
                    UdpSocket::bind("0.0.0.0:0").await?
                }
            };

            let message_bytes = serde_json::to_vec(&message)?;

            // Check message size
            if message_bytes.len() > 65507 {
                return Err("Message exceeds UDP packet size limit".into());
            }

            // Send the message
            socket.send_to(&message_bytes, address).await?;

            // Try to read response with timeout
            // Use longer timeout for encryption requests (10 seconds)
            let timeout_duration = match message {
                Message::EncryptionRequest { .. } => Duration::from_secs(10),
                _ => Duration::from_millis(500),
            };
            
            let mut buffer = vec![0u8; 65535];
            match tokio::time::timeout(timeout_duration, socket.recv_from(&mut buffer)).await {
                Ok(Ok((n, _))) => {
                    let response: Message = serde_json::from_slice(&buffer[..n])?;
                    Ok(Some(response))
                }
                _ => Ok(None),
            }
        } else {
            Err(format!("Unknown node ID: {}", node_id).into())
        }
    }

    /// Periodic failure simulation task
    async fn failure_simulation_task(&self) {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::from_entropy();
        let mut interval = interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            // Random chance to enter Failed state
            if rng.gen_bool(0.2) {
                // 20% chance every 30 seconds
                info!("[Node {}] *** Entering FAILED state ***", self.id);
                {
                    let mut state = self.state.write().await;
                    *state = NodeState::Failed;
                }

                // Stay in failed state for up to 20 seconds
                let failure_duration = Duration::from_secs(rng.gen_range(10..=20));
                sleep(failure_duration).await;

                info!("[Node {}] *** Entering RECOVERING state ***", self.id);
                {
                    let mut state = self.state.write().await;
                    *state = NodeState::Recovering;
                }

                // Perform state synchronization
                self.recover_state().await;

                info!("[Node {}] *** Returning to ACTIVE state ***", self.id);
                {
                    let mut state = self.state.write().await;
                    *state = NodeState::Active;
                }
            }
        }
    }

    /// Recover state from coordinator
    async fn recover_state(&self) {
        info!("[Node {}] Recovering state from peers...", self.id);

        // Query coordinator for state
        let manager = self.election_manager.lock().await;
        if let Some(coordinator_id) = manager.get_coordinator() {
            if coordinator_id != self.id {
                let message = Message::StateSync { from_node: self.id };
                if let Ok(Some(Message::StateSyncResponse { coordinator_id, .. })) =
                    self.send_message_to_node(coordinator_id, message).await
                {
                    info!(
                        "[Node {}] State synchronized with coordinator: Node {}",
                        self.id, coordinator_id
                    );
                }
            }
        }

        // Simulate recovery delay
        sleep(Duration::from_millis(500)).await;
    }

    /// Periodic election task
    async fn periodic_election_task(&self) {
        // Wait a bit before starting elections
        sleep(Duration::from_secs(5)).await;

        let mut interval = interval(Duration::from_secs(15));

        loop {
            interval.tick().await;

            let state = self.state.read().await;
            if *state != NodeState::Active {
                continue;
            }
            drop(state);

            // Trigger election
            self.trigger_election().await;
        }
    }

    /// Trigger an election
    async fn trigger_election(&self) {
        let current_load = *self.current_load.read().await;
        let current_processed = *self.processed_requests.read().await;
        let mut manager = self.election_manager.lock().await;

        // Collect load and processed counts from all nodes
        let mut all_loads = HashMap::new();
        let mut all_processed = HashMap::new();
        all_loads.insert(self.id, current_load);
        all_processed.insert(self.id, current_processed);

        for (&peer_id, _) in &self.peer_addresses {
            let message = Message::LoadQuery { from_node: self.id };
            if let Ok(Some(Message::LoadResponse { node_id, load, processed_count, .. })) =
                self.send_message_to_node(peer_id, message).await
            {
                all_loads.insert(node_id, load);
                all_processed.insert(node_id, processed_count);
            }
        }

        // Calculate total processed and percentages
        let total_processed: usize = all_processed.values().sum();
        
        // Find node with lowest load
        if let Some((&lowest_node, &lowest_load)) =
            all_loads.iter().min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        {
            let result = ElectionResult::new(lowest_node, lowest_load, all_loads);
            result.log_result();
            
            // Log work distribution percentages
            if total_processed > 0 {
                info!("=== WORK DISTRIBUTION ===");
                let mut sorted_nodes: Vec<_> = all_processed.iter().collect();
                sorted_nodes.sort_by(|a, b| b.1.cmp(a.1)); // Sort by processed count descending
                for (node_id, processed) in sorted_nodes {
                    let percentage = (*processed as f64 / total_processed as f64) * 100.0;
                    info!("  Node {}: {} requests ({:.1}%)", node_id, processed, percentage);
                }
                info!("  Total: {} requests", total_processed);
                info!("=========================");
            }

            if lowest_node == self.id {
                // This node should be coordinator
                let send_fn = |node: NodeId, msg: Message| {
                    let self_clone = self.clone();
                    tokio::spawn(async move {
                        let _ = self_clone.send_message_to_node(node, msg).await;
                    });
                    true
                };
                manager.announce_coordinator(current_load, send_fn);
            } else {
                // Update coordinator
                manager.update_coordinator(lowest_node, lowest_load);
            }
        }
    }

    /// Get current node statistics
    pub async fn get_stats(&self) -> NodeStats {
        NodeStats {
            id: self.id,
            state: self.state.read().await.clone(),
            load: *self.current_load.read().await,
            queue_length: *self.queue_length.read().await,
            processed_requests: *self.processed_requests.read().await,
            is_coordinator: self.election_manager.lock().await.is_coordinator(),
        }
    }
}

impl Clone for CloudNode {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            address: self.address.clone(),
            state: Arc::clone(&self.state),
            election_manager: Arc::clone(&self.election_manager),
            current_load: Arc::clone(&self.current_load),
            queue_length: Arc::clone(&self.queue_length),
            peer_addresses: self.peer_addresses.clone(),
            processed_requests: Arc::clone(&self.processed_requests),
            active_sessions: Arc::clone(&self.active_sessions),
            stored_images: Arc::clone(&self.stored_images),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeStats {
    pub id: NodeId,
    pub state: NodeState,
    pub load: f64,
    pub queue_length: usize,
    pub processed_requests: usize,
    pub is_coordinator: bool,
}
