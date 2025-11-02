use crate::chunking::{ChunkReassembler, ChunkedMessage};
use crate::election::{ElectionManager, ElectionResult};
use crate::encryption;
use crate::messages::{Message, NodeId, NodeState, ReceivedImageInfo};
use log::{debug, error, info, warn};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
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

/// Cached load information for a peer node
#[derive(Clone, Debug)]
pub struct CachedLoadInfo {
    pub load: f64,
    pub processed_count: usize,
    pub timestamp: Instant,
}

/// Cloud Node that participates in the distributed system
pub struct CloudNode {
    pub id: NodeId,
    pub address: String,
    pub state: Arc<RwLock<NodeState>>,
    pub election_manager: Arc<Mutex<ElectionManager>>,
    pub current_load: Arc<RwLock<f64>>,
    pub active_requests: Arc<RwLock<usize>>, // Number of requests currently being processed
    pub peer_addresses: HashMap<NodeId, String>,
    pub processed_requests: Arc<RwLock<usize>>, // Total completed (for metrics only)
    pub active_sessions: Arc<RwLock<HashMap<String, String>>>, // username -> client_id
    pub stored_images: Arc<RwLock<HashMap<String, Vec<StoredImage>>>>, // username -> list of images
    pub chunk_reassembler: Arc<Mutex<ChunkReassembler>>, // For reassembling multi-packet messages
    pub in_flight_requests: Arc<RwLock<HashSet<String>>>, // Track active request IDs to prevent duplicates
    pub chunk_cache: Arc<RwLock<HashMap<String, Vec<ChunkedMessage>>>>, // Cache sent chunks for retransmission
    pub last_heartbeat: Arc<RwLock<HashMap<NodeId, Instant>>>, // Track last heartbeat from each peer
    pub failed_nodes: Arc<RwLock<HashSet<NodeId>>>, // Nodes detected as failed
    pub peer_load_cache: Arc<RwLock<HashMap<NodeId, CachedLoadInfo>>>, // Cached load info from heartbeats
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
            active_requests: Arc::new(RwLock::new(0)),
            peer_addresses,
            processed_requests: Arc::new(RwLock::new(0)),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            stored_images: Arc::new(RwLock::new(HashMap::new())),
            chunk_reassembler: Arc::new(Mutex::new(ChunkReassembler::new())),
            in_flight_requests: Arc::new(RwLock::new(HashSet::new())),
            chunk_cache: Arc::new(RwLock::new(HashMap::new())),
            last_heartbeat: Arc::new(RwLock::new(HashMap::new())),
            failed_nodes: Arc::new(RwLock::new(HashSet::new())),
            peer_load_cache: Arc::new(RwLock::new(HashMap::new())),
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

        // NEW HEARTBEAT-BASED FAILURE DETECTION SYSTEM
        // Instead of periodic elections, use heartbeat monitoring

        // Perform initial election to establish first coordinator
        let self_clone = self.clone();
        tokio::spawn(async move {
            sleep(Duration::from_secs(5)).await; // Wait for all nodes to start
            self_clone.trigger_election().await;
        });

        // Start heartbeat sender (ping all peers every 2 seconds)
        let self_clone = self.clone();
        tokio::spawn(async move {
            self_clone.heartbeat_sender_task().await;
        });

        // Start failure detector (check for failed nodes every 3 seconds)
        let self_clone = self.clone();
        tokio::spawn(async move {
            self_clone.failure_detector_task().await;
        });

        // Start load monitoring (log load distribution every 10 seconds)
        let self_clone = self.clone();
        tokio::spawn(async move {
            self_clone.load_monitoring_task().await;
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

                    // Log active request count when new request arrives
                    let active_count = *self.active_requests.read().await;
                    if active_count > 0 {
                        debug!("[Node {}] Received new datagram ({} bytes) while {} requests active - spawning concurrent handler",
                               self.id, n, active_count);
                    }

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

        // Try to parse as ChunkedMessage first (for client communication)
        // If that fails, parse as direct Message (for node-to-node communication)
        let message: Message = match serde_json::from_slice::<ChunkedMessage>(&data) {
            Ok(chunked_message) => {
                // Check if it's a retransmit request
                if let ChunkedMessage::RetransmitRequest { chunk_id, missing_indices } = chunked_message {
                    info!("[Node {}] Received retransmit request for {} chunks (ID: {})", 
                          self.id, missing_indices.len(), &chunk_id[..8]);
                    
                    // Look up cached chunks
                    let cache = self.chunk_cache.read().await;
                    if let Some(cached_chunks) = cache.get(&chunk_id) {
                        info!("[Node {}] Retransmitting {} missing chunks", self.id, missing_indices.len());
                        
                        // Resend only the missing chunks
                        for &index in &missing_indices {
                            if let Some(chunk) = cached_chunks.get(index as usize) {
                                let chunk_bytes = serde_json::to_vec(chunk)?;
                                socket.send_to(&chunk_bytes, addr).await?;
                                debug!("[Node {}] Retransmitted chunk {}", self.id, index);
                                
                                // Small delay between retransmissions
                                tokio::time::sleep(Duration::from_millis(2)).await;
                            }
                        }
                        info!("[Node {}] Retransmission complete", self.id);
                    } else {
                        warn!("[Node {}] No cached chunks found for ID {}", self.id, chunk_id);
                    }
                    return Ok(());
                }
                
                // It's a normal chunked message from a client
                debug!("[Node {}] Received chunked message from {}", self.id, addr);

                // Try to reassemble
                let complete_data = {
                    let mut reassembler = self.chunk_reassembler.lock().await;
                    reassembler.process_chunk(chunked_message)
                };

                // If not complete yet, wait for more chunks
                let complete_data = match complete_data {
                    Some(data) => data,
                    None => {
                        debug!("[Node {}] Waiting for more chunks from {}", self.id, addr);
                        return Ok(());
                    }
                };

                // Parse complete message
                serde_json::from_slice(&complete_data)?
            }
            Err(_) => {
                // Not a chunked message, try parsing as direct Message (node-to-node)
                match serde_json::from_slice::<Message>(&data) {
                    Ok(msg) => {
                        debug!("[Node {}] Received direct message from peer", self.id);
                        msg
                    }
                    Err(e) => {
                        error!("[Node {}] Failed to parse message: {}", self.id, e);
                        return Err(e.into());
                    }
                }
            }
        };

        debug!("[Node {}] Received from {}: {}", self.id, addr, message);

        // Process message based on type
        let response = self.process_message(message, addr).await;

        // Send response if any
        if let Some(response) = response {
            let response_bytes = serde_json::to_vec(&response)?;

            debug!("[Node {}] Sending response: {} bytes", self.id, response_bytes.len());

            // Only use chunking for large responses (client messages with image data)
            // Node-to-node messages are small and sent directly
            let needs_chunking = matches!(
                response,
                Message::EncryptionResponse { .. } | 
                Message::DecryptionResponse { .. } | 
                Message::ViewImageResponse { .. }
            );

            if needs_chunking {
                // Fragment response for client
                let chunks = ChunkedMessage::fragment(response_bytes);

                debug!("[Node {}] Sending {} chunks to {}", self.id, chunks.len(), addr);

                // Cache chunks for potential retransmission
                if let ChunkedMessage::MultiPacket { ref chunk_id, .. } = chunks[0] {
                    let mut cache = self.chunk_cache.write().await;
                    cache.insert(chunk_id.clone(), chunks.clone());
                    debug!("[Node {}] Cached {} chunks with ID {}", self.id, chunks.len(), chunk_id);
                }

                // Send all chunks with delay to prevent UDP packet loss and buffer exhaustion
                for (i, chunk) in chunks.iter().enumerate() {
                    let chunk_bytes = serde_json::to_vec(&chunk)?;
                    socket.send_to(&chunk_bytes, addr).await?;

                        // Delay between chunks to prevent overwhelming receiver's socket buffer
                        // 10ms provides good balance between throughput and reliability
                        // Only delay if not the last chunk
                        if i < chunks.len() - 1 {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                }

                debug!("[Node {}] Sent {} chunks to {}", self.id, chunks.len(), addr);
            } else {
                // Send directly for node-to-node communication
                socket.send_to(&response_bytes, addr).await?;
                debug!("[Node {}] Sent direct response to {}", self.id, addr);
            }
        }

        Ok(())
    }

    /// Send response directly to client (bypassing coordinator)
    async fn send_response_to_client(
        &self,
        client_addr: SocketAddr,
        response: Message,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary socket for sending (or reuse a dedicated socket)
        let socket = UdpSocket::bind("0.0.0.0:0").await?;

        let response_bytes = serde_json::to_vec(&response)?;

        // Check if response needs chunking (large responses with image data)
        let needs_chunking = matches!(
            response,
            Message::EncryptionResponse { .. } |
            Message::DecryptionResponse { .. } |
            Message::ViewImageResponse { .. }
        );

        if needs_chunking {
            // Fragment response for client
            let chunks = ChunkedMessage::fragment(response_bytes);

            info!("[Node {}] Sending {} chunks directly to client {}",
                  self.id, chunks.len(), client_addr);

            // Send all chunks with delay to prevent UDP packet loss
            for (i, chunk) in chunks.iter().enumerate() {
                let chunk_bytes = serde_json::to_vec(&chunk)?;
                socket.send_to(&chunk_bytes, client_addr).await?;

                // Delay between chunks to prevent overwhelming receiver's socket buffer
                if i < chunks.len() - 1 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        } else {
            // Small response - send directly
            socket.send_to(&response_bytes, client_addr).await?;
        }

        Ok(())
    }

    /// Process incoming message
    async fn process_message(&self, message: Message, addr: SocketAddr) -> Option<Message> {
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
                client_address,
            } => {
                // Check if this request is already being processed (deduplication)
                {
                    let mut in_flight = self.in_flight_requests.write().await;
                    if in_flight.contains(&request_id) {
                        if !forwarded {
                            // Only ignore non-forwarded duplicates
                            // Forwarded requests from coordinator MUST be processed even if duplicate
                            warn!("[Node {}] Ignoring duplicate request {} (already in flight)", self.id, request_id);
                            return None;
                        } else {
                            // Coordinator has selected us to process this - override duplicate detection
                            info!("[Node {}] Processing coordinator-forwarded request {} despite duplicate (coordinator override)",
                                  self.id, request_id);
                        }
                    } else {
                        // Mark request as in-flight
                        in_flight.insert(request_id.clone());
                    }
                }

                // Increment active requests at the START of handling (whether processing or forwarding)
                // This tracks all active tasks on this node
                {
                    let mut active = self.active_requests.write().await;
                    *active += 1;
                    let mut load = self.current_load.write().await;
                    *load = *active as f64;
                    info!("[Node {}] Handling request {} (active requests now: {})",
                          self.id, request_id, *active);
                }

                // Process request and ensure cleanup happens regardless of outcome
                let response = if forwarded {
                    // Request forwarded by coordinator - MUST process locally
                    info!("[Node {}] Processing forwarded request {} locally (from coordinator)", self.id, request_id);

                    // Process encryption (active_requests incremented inside process_encryption_request)
                    let self_clone = Arc::new(self.clone());
                    let result = self_clone
                        .process_encryption_request(request_id.clone(), image_data, usernames, quota)
                        .await;

                    // If we have a client_address, send response directly to client
                    if let Some(ref client_addr_str) = client_address {
                        info!("[Node {}] Sending response for {} directly to client at {}",
                              self.id, request_id, client_addr_str);

                        // Parse client address and send response directly
                        match client_addr_str.parse::<SocketAddr>() {
                            Ok(client_sock_addr) => {
                                // Send response directly to client
                                if let Err(e) = self_clone.send_response_to_client(client_sock_addr, result.clone()).await {
                                    error!("[Node {}] Failed to send direct response to client {}: {}",
                                           self.id, client_addr_str, e);
                                    // Still return response to coordinator as fallback
                                    Some(result)
                                } else {
                                    info!("[Node {}] Successfully sent response for {} directly to client",
                                          self.id, request_id);
                                    // Return None - no need to send back through coordinator
                                    None
                                }
                            }
                            Err(e) => {
                                error!("[Node {}] Failed to parse client address '{}': {}",
                                       self.id, client_addr_str, e);
                                // Fall back to normal response flow
                                Some(result)
                            }
                        }
                    } else {
                        // No client address - return response normally
                        Some(result)
                    }
                } else {
                    // Get current coordinator (may change due to elections)
                    let manager = self.election_manager.lock().await;
                    let coordinator_id = manager.get_coordinator().unwrap_or(self.id);
                    drop(manager); // Release lock immediately

                    if coordinator_id != self.id {
                        // Not coordinator - forward to current coordinator for load balancing
                        info!("[Node {}] Forwarding request {} to coordinator Node {} for load balancing",
                              self.id, request_id, coordinator_id);

                        // Capture client address if not already set (original request from client)
                        let client_addr = if client_address.is_none() {
                            // Check if this is from a client (not from a peer node)
                            let addr_str = addr.to_string();
                            let is_peer = self.peer_addresses.values().any(|peer_addr| {
                                // peer_addr is a String like "10.40.59.43:8001"
                                // Compare with sender's address
                                peer_addr == &addr_str
                            });

                            if !is_peer {
                                Some(addr_str)
                            } else {
                                None
                            }
                        } else {
                            client_address
                        };

                        let forward_message = Message::EncryptionRequest {
                            request_id: request_id.clone(),
                            client_username,
                            image_data,
                            usernames,
                            quota,
                            forwarded: false, // Coordinator will do load balancing
                            client_address: client_addr,
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
                                Some(Message::EncryptionResponse {
                                    request_id: request_id.clone(),
                                    encrypted_image: vec![],
                                    success: false,
                                    error: Some("Coordinator did not respond".to_string()),
                                })
                            }
                            Err(e) => {
                                error!("[Node {}] Failed to contact coordinator Node {} for {}: {}",
                                       self.id, coordinator_id, request_id, e);
                                Some(Message::EncryptionResponse {
                                    request_id: request_id.clone(),
                                    encrypted_image: vec![],
                                    success: false,
                                    error: Some(format!("Coordinator unreachable: {}", e)),
                                })
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

                            // Process encryption (active_requests managed inside process_encryption_request)
                            let self_clone = Arc::new(self.clone());
                            let result = self_clone
                                .process_encryption_request(request_id.clone(), image_data, usernames, quota)
                                .await;

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
                                client_address, // Pass through client address for direct response
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
                                        request_id: request_id.clone(),
                                        encrypted_image: vec![],
                                        success: false,
                                        error: Some("Selected node did not respond".to_string()),
                                    })
                                }
                                Err(e) => {
                                    error!("[Node {}] Failed to forward to Node {} for {}: {}",
                                           self.id, lowest_load_node, request_id, e);
                                    Some(Message::EncryptionResponse {
                                        request_id: request_id.clone(),
                                        encrypted_image: vec![],
                                        success: false,
                                        error: Some(format!("Forward to selected node failed: {}", e)),
                                    })
                                }
                            }
                        }
                    }
                };

                // Remove request from in-flight set now that it's complete
                {
                    let mut in_flight = self.in_flight_requests.write().await;
                    in_flight.remove(&request_id);
                }

                // Decrement active requests now that handling is complete
                {
                    let mut active = self.active_requests.write().await;
                    *active = active.saturating_sub(1);
                    let mut load = self.current_load.write().await;
                    *load = *active as f64;
                    info!("[Node {}] Finished handling request {} (active requests now: {})",
                          self.id, request_id, *active);
                }

                response
            }

            Message::DecryptionRequest {
                request_id,
                client_username: _,
                encrypted_image,
                usernames: _,
                quota: _,
            } => {
                // Decryption is fast and doesn't require load balancing
                // Process locally on whatever node receives the request
                info!("[Node {}] Processing decryption request: {}", self.id, request_id);

                let self_clone = Arc::new(self.clone());
                let result = self_clone
                    .process_decryption_request(request_id.clone(), encrypted_image)
                    .await;

                Some(result)
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
                let active = *self.active_requests.read().await;
                let processed = *self.processed_requests.read().await;
                Some(Message::LoadResponse {
                    node_id: self.id,
                    load,
                    queue_length: active, // Report active requests as "queue"
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

            Message::Heartbeat { from_node, load, processed_count } => {
                // Record that we received a heartbeat from this node
                {
                    let now = Instant::now();
                    let mut heartbeats = self.last_heartbeat.write().await;
                    heartbeats.insert(from_node, now);

                    // Cache the load information
                    let mut load_cache = self.peer_load_cache.write().await;
                    load_cache.insert(from_node, CachedLoadInfo {
                        load,
                        processed_count,
                        timestamp: now,
                    });

                    // If this node was marked as failed, remove it from failed set
                    let mut failed = self.failed_nodes.write().await;
                    if failed.remove(&from_node) {
                        info!("[Node {}] Node {} recovered (heartbeat received)", self.id, from_node);
                    }
                }

                // Send acknowledgment with our current load
                let my_load = *self.current_load.read().await;
                let my_processed = *self.processed_requests.read().await;
                Some(Message::HeartbeatAck {
                    from_node: self.id,
                    load: my_load,
                    processed_count: my_processed,
                })
            }

            Message::HeartbeatAck { from_node, load, processed_count } => {
                // Update last heartbeat time for this node
                {
                    let now = Instant::now();
                    let mut heartbeats = self.last_heartbeat.write().await;
                    heartbeats.insert(from_node, now);

                    // Cache the load information
                    let mut load_cache = self.peer_load_cache.write().await;
                    load_cache.insert(from_node, CachedLoadInfo {
                        load,
                        processed_count,
                        timestamp: now,
                    });

                    // If this node was marked as failed, remove it from failed set
                    let mut failed = self.failed_nodes.write().await;
                    if failed.remove(&from_node) {
                        info!("[Node {}] Node {} recovered (heartbeat ack)", self.id, from_node);
                    }
                }
                None // No response needed
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
        let start_time = Instant::now();

        // Get current active count for logging (active_requests already incremented by caller)
        let active_count = *self.active_requests.read().await;

        info!(
            "[Node {}] START encrypting request {} (current active: {})",
            self.id, request_id, active_count
        );

        // Perform encryption
        let result = match encryption::encrypt_image(image_data, usernames, quota).await {
            Ok(encrypted_image) => {
                let mut processed = self.processed_requests.write().await;
                *processed += 1;

                info!(
                    "[Node {}] Successfully encrypted request: {} (total: {})",
                    self.id, request_id, *processed
                );

                Message::EncryptionResponse {
                    request_id: request_id.clone(),
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
                    request_id: request_id.clone(),
                    encrypted_image: vec![],
                    success: false,
                    error: Some(e),
                }
            }
        };

        let elapsed = start_time.elapsed();
        info!(
            "[Node {}] FINISH encrypting request {} in {:.2}s",
            self.id, request_id, elapsed.as_secs_f64()
        );

        result
    }

    async fn process_decryption_request(
        &self,
        request_id: String,
        encrypted_image: Vec<u8>,
    ) -> Message {
        info!(
            "[Node {}] Processing decryption request: {}",
            self.id, request_id
        );

        // Perform decryption
        match encryption::decrypt_image(encrypted_image).await {
            Ok((decrypted_image, _metadata)) => {
                info!(
                    "[Node {}] Successfully decrypted request: {}",
                    self.id, request_id
                );

                Message::DecryptionResponse {
                    request_id,
                    decrypted_image,
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                error!(
                    "[Node {}] Decryption failed for request {}: {}",
                    self.id, request_id, e
                );

                Message::DecryptionResponse {
                    request_id,
                    decrypted_image: vec![],
                    success: false,
                    error: Some(e),
                }
            }
        }
    }

    /// Find the node with the lowest load (including self)
    /// Uses hybrid scoring: 70% current load + 30% historical work percentage
    /// This ensures fair distribution over time while still being responsive to current load
    ///
    /// OPTIMIZED: Uses cached load data from heartbeats instead of querying every node
    /// This dramatically reduces network overhead (from N queries per request to 0)
    async fn find_lowest_load_node(&self) -> NodeId {
        let my_load = *self.current_load.read().await;
        let my_processed = *self.processed_requests.read().await;

        debug!("[Node {}] Finding lowest load node (my load: {:.2}, processed: {})",
               self.id, my_load, my_processed);

        // Collect data from all nodes (including self)
        let mut node_data: HashMap<NodeId, (f64, usize)> = HashMap::new();
        node_data.insert(self.id, (my_load, my_processed));

        // Get list of failed nodes to skip them
        let failed = self.failed_nodes.read().await.clone();

        // Get cached load data from heartbeats
        let load_cache = self.peer_load_cache.read().await;
        let now = Instant::now();
        const CACHE_TTL: Duration = Duration::from_secs(5); // Consider cache stale after 5 seconds

        for (peer_id, _) in &self.peer_addresses {
            // Skip failed nodes
            if failed.contains(peer_id) {
                debug!("[Node {}] Skipping failed Node {} in load balancing", self.id, peer_id);
                continue;
            }

            // Try to use cached data first
            if let Some(cached) = load_cache.get(peer_id) {
                let age = now.duration_since(cached.timestamp);

                if age < CACHE_TTL {
                    // Cache is fresh - use it!
                    debug!("[Node {}] Using cached load for Node {} (age: {:.1}s, load: {:.2})",
                           self.id, peer_id, age.as_secs_f64(), cached.load);
                    node_data.insert(*peer_id, (cached.load, cached.processed_count));
                } else {
                    // Cache is stale - log it but still use it as fallback
                    debug!("[Node {}] Stale cache for Node {} (age: {:.1}s), using anyway",
                           self.id, peer_id, age.as_secs_f64());
                    node_data.insert(*peer_id, (cached.load, cached.processed_count));
                }
            } else {
                // No cached data - this node might not have sent heartbeat yet
                // Use conservative estimate (assume moderate load)
                debug!("[Node {}] No cached data for Node {}, assuming moderate load",
                       self.id, peer_id);
                node_data.insert(*peer_id, (my_load, 0)); // Assume similar load to self
            }
        }

        drop(load_cache);
        drop(failed);

        // Calculate total processed requests across all nodes
        let total_processed: usize = node_data.values().map(|(_, p)| p).sum();

        // Find node with best score (lowest combined metric)
        let mut best_node = self.id;
        let mut best_score = f64::MAX;

        for (node_id, (load, processed)) in &node_data {
            // Calculate historical work percentage (0.0 to 1.0)
            let work_percentage = if total_processed > 0 {
                *processed as f64 / total_processed as f64
            } else {
                0.0 // All nodes at 0%, treat equally
            };

            // Hybrid score: 70% current load + 30% historical work percentage
            // This balances immediate responsiveness with long-term fairness
            let score = (0.7 * load) + (0.3 * work_percentage * 100.0);

            debug!("[Node {}] Node {} score: {:.2} (load: {:.2}, work%: {:.1}%)",
                   self.id, node_id, score, load, work_percentage * 100.0);

            if score < best_score {
                best_score = score;
                best_node = *node_id;
            }
        }

        info!("[Node {}] Selected node: {} (score: {:.2}) [CACHED]", self.id, best_node, best_score);
        best_node
    }

    /// Send message to another node with retry logic
    async fn send_message_to_node(&self, node_id: NodeId, message: Message) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        // Determine if this message type should be retried
        let should_retry = matches!(message,
            Message::LoadQuery { .. } |
            Message::Election { .. } |
            Message::Coordinator { .. }
        );

        let max_attempts = if should_retry { 3 } else { 1 };

        for attempt in 1..=max_attempts {
            let should_retry = {
                let result = self.send_message_to_node_once(node_id, message.clone()).await;

                match result {
                    Ok(Some(response)) => return Ok(Some(response)),
                    Ok(None) if attempt < max_attempts => {
                        debug!("[Node {}] No response from Node {} on attempt {}/{}, retrying...",
                               self.id, node_id, attempt, max_attempts);
                        true // Should retry
                    }
                    Ok(None) => {
                        debug!("[Node {}] No response from Node {} after {} attempts",
                               self.id, node_id, max_attempts);
                        return Ok(None);
                    }
                    Err(e) if attempt < max_attempts => {
                        debug!("[Node {}] Error communicating with Node {} on attempt {}/{}: {}, retrying...",
                               self.id, node_id, attempt, max_attempts, e);
                        true // Should retry
                    }
                    Err(e) => return Err(e),
                }
                // result is dropped here at the end of the block
            };

            // Sleep only if we determined we should retry (result is already dropped)
            if should_retry {
                tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
            }
        }

        Ok(None)
    }

    /// Send message to another node (single attempt)
    async fn send_message_to_node_once(&self, node_id: NodeId, message: Message) -> Result<Option<Message>, Box<dyn std::error::Error>> {
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

            // Determine if message needs chunking
            let needs_chunking = matches!(message,
                Message::EncryptionRequest { .. } |
                Message::EncryptionResponse { .. } |
                Message::DecryptionRequest { .. } |
                Message::DecryptionResponse { .. }
            );

            // Use appropriate timeouts based on message type
            let timeout_duration = match message {
                Message::EncryptionRequest { .. } | Message::DecryptionRequest { .. } => Duration::from_secs(30),
                Message::LoadQuery { .. } => Duration::from_secs(3), // Increased from 500ms - nodes may be busy
                Message::Election { .. } | Message::Coordinator { .. } => Duration::from_secs(2), // Critical messages
                _ => Duration::from_secs(1), // Default 1 second for other messages
            };

            if needs_chunking && message_bytes.len() > 45000 {
                // Use chunking for large messages
                let chunks = ChunkedMessage::fragment(message_bytes);

                // Send all chunks
                for (i, chunk) in chunks.iter().enumerate() {
                    let chunk_bytes = serde_json::to_vec(&chunk)?;
                    socket.send_to(&chunk_bytes, address).await?;

                    // Delay between chunks to prevent buffer exhaustion and packet loss
                    // 2ms prevents "No buffer space available" errors (OS error 105)
                    if i < chunks.len() - 1 {
                        tokio::time::sleep(Duration::from_millis(2)).await;
                    }
                }

                // Receive and reassemble chunked response
                let mut chunk_buffer = vec![0u8; 65535];
                let mut reassembler = ChunkReassembler::new();

                loop {
                    match tokio::time::timeout(timeout_duration, socket.recv_from(&mut chunk_buffer)).await {
                        Ok(Ok((n, _))) => {
                            // Try to parse as chunked message
                            if let Ok(chunk_msg) = serde_json::from_slice::<ChunkedMessage>(&chunk_buffer[..n]) {
                                if let Some(complete_data) = reassembler.process_chunk(chunk_msg) {
                                    // Got complete message
                                    let response: Message = serde_json::from_slice(&complete_data)?;
                                    debug!("[Node {}] Received response from Node {}: {}", self.id, node_id, response);
                                    return Ok(Some(response));
                                }
                                // Continue receiving more chunks
                            } else {
                                // Not a chunked message, try parsing directly
                                if let Ok(response) = serde_json::from_slice::<Message>(&chunk_buffer[..n]) {
                                    debug!("[Node {}] Received response from Node {}: {}", self.id, node_id, response);
                                    return Ok(Some(response));
                                } else {
                                    debug!("[Node {}] Received invalid message from Node {} ({} bytes)", self.id, node_id, n);
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            debug!("[Node {}] Socket error waiting for response from Node {}: {}", self.id, node_id, e);
                            return Ok(None);
                        }
                        Err(_) => {
                            debug!("[Node {}] Timeout waiting for response from Node {} after {:?}",
                                   self.id, node_id, timeout_duration);
                            return Ok(None);
                        }
                    }
                }
            } else {
                // Small message - send directly without chunking
                if message_bytes.len() > 65507 {
                    return Err("Message exceeds UDP packet size limit".into());
                }

                socket.send_to(&message_bytes, address).await?;

                // Receive response (might be chunked)
                let mut chunk_buffer = vec![0u8; 65535];
                let mut reassembler = ChunkReassembler::new();

                loop {
                    match tokio::time::timeout(timeout_duration, socket.recv_from(&mut chunk_buffer)).await {
                        Ok(Ok((n, _))) => {
                            // Try to parse as chunked message first
                            if let Ok(chunk_msg) = serde_json::from_slice::<ChunkedMessage>(&chunk_buffer[..n]) {
                                if let Some(complete_data) = reassembler.process_chunk(chunk_msg) {
                                    // Got complete message
                                    let response: Message = serde_json::from_slice(&complete_data)?;
                                    debug!("[Node {}] Received response from Node {}: {}", self.id, node_id, response);
                                    return Ok(Some(response));
                                }
                                // Continue receiving more chunks
                            } else {
                                // Not a chunked message, try parsing directly
                                if let Ok(response) = serde_json::from_slice::<Message>(&chunk_buffer[..n]) {
                                    debug!("[Node {}] Received response from Node {}: {}", self.id, node_id, response);
                                    return Ok(Some(response));
                                } else {
                                    debug!("[Node {}] Received invalid message from Node {} ({} bytes)", self.id, node_id, n);
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            debug!("[Node {}] Socket error waiting for response from Node {}: {}", self.id, node_id, e);
                            return Ok(None);
                        }
                        Err(_) => {
                            debug!("[Node {}] Timeout waiting for response from Node {} after {:?}",
                                   self.id, node_id, timeout_duration);
                            return Ok(None);
                        }
                    }
                }
            }
        } else {
            Err(format!("Unknown node ID: {}", node_id).into())
        }
    }

    /// Periodic failure simulation task
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    /// Heartbeat sender task - sends heartbeat to all peers every 2 seconds
    async fn heartbeat_sender_task(&self) {
        // Wait a bit for other nodes to start
        sleep(Duration::from_secs(3)).await;

        // Create a dedicated socket for heartbeats (reuse it instead of creating new ones)
        let bind_addr = format!("0.0.0.0:{}", 10000 + self.id);
        let heartbeat_socket = match UdpSocket::bind(&bind_addr).await {
            Ok(s) => s,
            Err(_) => {
                // Fallback to any available port
                match UdpSocket::bind("0.0.0.0:0").await {
                    Ok(s) => s,
                    Err(e) => {
                        error!("[Node {}] Failed to create heartbeat socket: {}", self.id, e);
                        return;
                    }
                }
            }
        };

        let mut interval = interval(Duration::from_secs(2));

        loop {
            interval.tick().await;

            // Get current load and processed count to include in heartbeat
            let current_load = *self.current_load.read().await;
            let current_processed = *self.processed_requests.read().await;

            // Send heartbeat to all peers
            for (peer_id, peer_addr) in &self.peer_addresses {
                let message = Message::Heartbeat {
                    from_node: self.id,
                    load: current_load,
                    processed_count: current_processed,
                };

                match serde_json::to_vec(&message) {
                    Ok(message_bytes) => {
                        match heartbeat_socket.send_to(&message_bytes, peer_addr).await {
                            Ok(_) => {
                                // Heartbeat sent successfully (silent success)
                            }
                            Err(e) => {
                                // Log error but continue - temporary network issues shouldn't crash the task
                                warn!("[Node {}] Failed to send heartbeat to Node {} ({}): {}",
                                      self.id, peer_id, peer_addr, e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("[Node {}] Failed to serialize heartbeat: {}", self.id, e);
                    }
                }
            }
        }
    }

    /// Failure detector task - checks for failed nodes every 3 seconds
    async fn failure_detector_task(&self) {
        // Wait longer for heartbeats to start flowing and all nodes to be ready
        // This prevents false-positive failure detection at startup
        sleep(Duration::from_secs(15)).await;

        let mut interval = interval(Duration::from_secs(3));
        const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(10); // Increased from 6s to 10s for more forgiveness

        loop {
            interval.tick().await;

            let now = Instant::now();
            let mut newly_failed_nodes = Vec::new();

            // Check all peers for heartbeat timeout
            {
                let heartbeats = self.last_heartbeat.read().await;
                let failed = self.failed_nodes.read().await;

                for (&peer_id, _) in &self.peer_addresses {
                    // Skip if already marked as failed
                    if failed.contains(&peer_id) {
                        continue;
                    }

                    // Check if we've received a heartbeat from this peer
                    if let Some(&last_seen) = heartbeats.get(&peer_id) {
                        let elapsed = now.duration_since(last_seen);
                        if elapsed > HEARTBEAT_TIMEOUT {
                            debug!("[Node {}] Node {} missed heartbeat (last seen {:.1}s ago)",
                                   self.id, peer_id, elapsed.as_secs_f64());
                            newly_failed_nodes.push(peer_id);
                        }
                    } else {
                        // Never received a heartbeat from this peer - may not have started yet
                        // Only mark as failed if enough time has passed since our startup
                        // (we start checking after 8 seconds, so this is reasonable)
                        debug!("[Node {}] Node {} never sent heartbeat (may not be started)", self.id, peer_id);
                    }
                }
            }

            // Handle newly detected failures
            if !newly_failed_nodes.is_empty() {
                let mut failed = self.failed_nodes.write().await;

                for failed_node in newly_failed_nodes {
                    failed.insert(failed_node);
                    warn!("[Node {}] FAILURE DETECTED: Node {} is not responding", self.id, failed_node);

                    // Check if the failed node is the coordinator
                    let manager = self.election_manager.lock().await;
                    let coordinator_id = manager.get_coordinator();
                    drop(manager);

                    if coordinator_id == Some(failed_node) {
                        // COORDINATOR FAILED - trigger election!
                        error!("[Node {}] COORDINATOR Node {} has FAILED! Triggering election...", self.id, failed_node);
                        drop(failed); // Release lock before triggering election
                        self.trigger_election().await;
                        break; // Exit loop to avoid multiple elections
                    }
                    // If non-coordinator node failed, just log it
                    // The coordinator will notice when it tries to load balance to this node
                }
            }
        }
    }

    /// Load monitoring task - logs load distribution every 10 seconds
    async fn load_monitoring_task(&self) {
        // Wait for system to stabilize before starting monitoring
        sleep(Duration::from_secs(20)).await;

        let mut interval = interval(Duration::from_secs(10));

        loop {
            interval.tick().await;

            // Get current coordinator
            let manager = self.election_manager.lock().await;
            let coordinator_id = manager.get_coordinator();
            drop(manager);

            // Get failed nodes
            let failed = self.failed_nodes.read().await;

            // Collect load information from all nodes
            let mut load_info = Vec::new();

            // Add this node's info
            let self_load = self.current_load.read().await;
            let self_processed = self.processed_requests.read().await;
            let self_active = self.active_requests.read().await;

            let is_coordinator = coordinator_id == Some(self.id);
            let status = if is_coordinator { "COORDINATOR" } else { "Worker" };

            load_info.push((
                self.id,
                *self_load,
                *self_processed,
                *self_active,
                status.to_string(),
            ));

            drop(self_load);
            drop(self_processed);
            drop(self_active);

            // Query all peer nodes for their load
            for (&peer_id, peer_addr) in &self.peer_addresses {
                // Skip failed nodes
                if failed.contains(&peer_id) {
                    load_info.push((peer_id, 0.0, 0, 0, "FAILED".to_string()));
                    continue;
                }

                // Send load query
                let message = Message::LoadQuery { from_node: self.id };

                match serde_json::to_vec(&message) {
                    Ok(message_bytes) => {
                        // Create temporary socket for query
                        match UdpSocket::bind("0.0.0.0:0").await {
                            Ok(query_socket) => {
                                // Set short timeout for monitoring query
                                match tokio::time::timeout(
                                    Duration::from_secs(2),
                                    async {
                                        // Send query
                                        query_socket.send_to(&message_bytes, peer_addr).await?;

                                        // Wait for response
                                        let mut buf = vec![0u8; 65535];
                                        let (n, _) = query_socket.recv_from(&mut buf).await?;
                                        let response: Message = serde_json::from_slice(&buf[..n])?;

                                        Ok::<Message, Box<dyn std::error::Error>>(response)
                                    }
                                ).await {
                                    Ok(Ok(Message::LoadResponse { node_id, load, queue_length, processed_count })) => {
                                        let status = if coordinator_id == Some(node_id) {
                                            "COORDINATOR"
                                        } else {
                                            "Worker"
                                        };
                                        load_info.push((node_id, load, processed_count, queue_length, status.to_string()));
                                    }
                                    _ => {
                                        // Query failed or timed out
                                        load_info.push((peer_id, 0.0, 0, 0, "NO_RESPONSE".to_string()));
                                    }
                                }
                            }
                            Err(_) => {
                                // Socket creation failed
                                load_info.push((peer_id, 0.0, 0, 0, "ERROR".to_string()));
                            }
                        }
                    }
                    Err(_) => {
                        // Serialization failed
                        load_info.push((peer_id, 0.0, 0, 0, "ERROR".to_string()));
                    }
                }
            }

            drop(failed);

            // Sort by node ID for consistent display
            load_info.sort_by_key(|(id, _, _, _, _)| *id);

            // Log the load distribution
            info!("[Node {}] ════════════════ CLUSTER LOAD DISTRIBUTION ════════════════", self.id);
            info!("[Node {}] ┌────────┬────────────┬───────────┬────────────┬──────────────┐", self.id);
            info!("[Node {}] │ Node   │ Status     │ Load      │ Processed  │ Active Reqs  │", self.id);
            info!("[Node {}] ├────────┼────────────┼───────────┼────────────┼──────────────┤", self.id);

            for (node_id, load, processed, active, status) in load_info {
                info!("[Node {}] │ Node {} │ {:10} │ {:8.2}% │ {:10} │ {:12} │",
                      self.id, node_id, status, load * 100.0, processed, active);
            }

            info!("[Node {}] └────────┴────────────┴───────────┴────────────┴──────────────┘", self.id);
        }
    }

    /// Periodic election task (DEPRECATED - now only used for initial election)
    /// Elections are now triggered only on coordinator failure
    #[allow(dead_code)]
    async fn periodic_election_task(&self) {
        // Wait a bit before starting elections
        sleep(Duration::from_secs(5)).await;

        // Increased from 15s to 60s to reduce coordinator churn
        let mut interval = interval(Duration::from_secs(60));

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

        // Get list of failed nodes to skip them in election
        let failed = self.failed_nodes.read().await;

        for (&peer_id, _) in &self.peer_addresses {
            // Skip failed nodes - they cannot be elected as coordinator
            if failed.contains(&peer_id) {
                info!("[Node {}] Skipping failed Node {} in election", self.id, peer_id);
                continue;
            }

            let message = Message::LoadQuery { from_node: self.id };
            if let Ok(Some(Message::LoadResponse { node_id, load, processed_count, .. })) =
                self.send_message_to_node(peer_id, message).await
            {
                all_loads.insert(node_id, load);
                all_processed.insert(node_id, processed_count);
            }
        }

        drop(failed); // Release read lock

        // Calculate total processed and percentages
        let total_processed: usize = all_processed.values().sum();
        
        // Find node with lowest load
        if let Some((&lowest_node, &lowest_load)) =
            all_loads.iter().min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        {
            let result = ElectionResult::new(lowest_node, lowest_load, all_loads.clone());
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

            // Add hysteresis: only change coordinator if load difference is significant
            // This prevents rapid coordinator changes due to minor load fluctuations
            let current_coordinator = manager.get_coordinator();
            let should_change = if let Some(current_coord) = current_coordinator {
                if current_coord == lowest_node {
                    // Already the right coordinator
                    false
                } else if let Some(&current_coord_load) = all_loads.get(&current_coord) {
                    // Only change if the new coordinator has significantly lower load (>20% difference)
                    let load_diff_ratio = (current_coord_load - lowest_load) / current_coord_load.max(0.01);
                    if load_diff_ratio > 0.20 {
                        info!("[Node {}] Coordinator change justified: current load {:.2}, new load {:.2} ({:.1}% improvement)",
                              self.id, current_coord_load, lowest_load, load_diff_ratio * 100.0);
                        true
                    } else {
                        info!("[Node {}] Skipping coordinator change: load difference {:.1}% is below 20% threshold",
                              self.id, load_diff_ratio * 100.0);
                        false
                    }
                } else {
                    // Current coordinator not in load list (may have failed), change
                    true
                }
            } else {
                // No coordinator yet, elect one
                true
            };

            if should_change {
                if lowest_node == self.id {
                    // This node should be coordinator - announce to all
                    let send_fn = |node: NodeId, msg: Message| {
                        let self_clone = self.clone();
                        tokio::spawn(async move {
                            let _ = self_clone.send_message_to_node(node, msg).await;
                        });
                        true
                    };
                    manager.announce_coordinator(current_load, send_fn);
                } else {
                    // Another node should be coordinator - update locally AND broadcast to all nodes
                    manager.update_coordinator(lowest_node, lowest_load);

                    info!("[Node {}] Broadcasting coordinator decision: Node {} with load {:.2}",
                          self.id, lowest_node, lowest_load);

                    // Broadcast coordinator message to ALL nodes (including the winner and this node)
                    // This ensures everyone has the same view
                    for (&peer_id, _) in &self.peer_addresses {
                        let message = Message::Coordinator {
                            node_id: lowest_node,
                            load: lowest_load,
                        };
                        let self_clone = self.clone();
                        tokio::spawn(async move {
                            let _ = self_clone.send_message_to_node(peer_id, message).await;
                        });
                    }
                }
            }
        }
    }

    /// Get current node statistics
    pub async fn get_stats(&self) -> NodeStats {
        NodeStats {
            id: self.id,
            state: self.state.read().await.clone(),
            load: *self.current_load.read().await,
            queue_length: *self.active_requests.read().await,
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
            active_requests: Arc::clone(&self.active_requests),
            peer_addresses: self.peer_addresses.clone(),
            processed_requests: Arc::clone(&self.processed_requests),
            active_sessions: Arc::clone(&self.active_sessions),
            stored_images: Arc::clone(&self.stored_images),
            chunk_reassembler: Arc::clone(&self.chunk_reassembler),
            in_flight_requests: Arc::clone(&self.in_flight_requests),
            chunk_cache: Arc::clone(&self.chunk_cache),
            last_heartbeat: Arc::clone(&self.last_heartbeat),
            failed_nodes: Arc::clone(&self.failed_nodes),
            peer_load_cache: Arc::clone(&self.peer_load_cache),
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
