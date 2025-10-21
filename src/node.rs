use crate::election::{ElectionManager, ElectionResult};
use crate::encryption;
use crate::messages::{Message, NodeId, NodeState};
use log::{debug, error, info};
use rand::Rng;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, sleep};

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
        }
    }

    /// Start the cloud node server
    pub async fn start(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        info!("[Node {}] Starting on {}", self.id, self.address);

        let listener = TcpListener::bind(&self.address).await?;
        info!("[Node {}] Listening on {}", self.id, self.address);

        // Start background tasks
        let self_clone = self.clone();
        tokio::spawn(async move {
            self_clone.failure_simulation_task().await;
        });

        let self_clone = self.clone();
        tokio::spawn(async move {
            self_clone.periodic_election_task().await;
        });

        // Accept incoming connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let self_clone = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = self_clone.handle_connection(stream, addr).await {
                            error!("[Node {}] Error handling connection: {}", self_clone.id, e);
                        }
                    });
                }
                Err(e) => {
                    error!("[Node {}] Error accepting connection: {}", self.id, e);
                }
            }
        }
    }

    /// Handle incoming connection
    async fn handle_connection(
        &self,
        mut stream: TcpStream,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if node is in Failed state
        let state = self.state.read().await;
        if *state == NodeState::Failed {
            debug!("[Node {}] Ignoring connection (FAILED state)", self.id);
            return Ok(());
        }
        drop(state);

        let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer
        let n = stream.read(&mut buffer).await?;

        if n == 0 {
            return Ok(());
        }

        let message: Message = serde_json::from_slice(&buffer[..n])?;
        debug!("[Node {}] Received from {}: {}", self.id, addr, message);

        // Process message based on type
        let response = self.process_message(message).await;

        if let Some(response) = response {
            let response_bytes = serde_json::to_vec(&response)?;
            stream.write_all(&response_bytes).await?;
        }

        Ok(())
    }

    /// Process incoming message
    async fn process_message(&self, message: Message) -> Option<Message> {
        match message {
            Message::EncryptionRequest {
                request_id,
                image_data,
                usernames,
                quota,
            } => {
                // Increment queue length
                {
                    let mut queue = self.queue_length.write().await;
                    *queue += 1;
                }

                // Process encryption in a separate task
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
                Some(Message::LoadResponse {
                    node_id: self.id,
                    load,
                    queue_length: queue,
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

        // Update load (simulate based on queue length and processing)
        {
            let queue = *self.queue_length.read().await;
            let mut load = self.current_load.write().await;
            *load = (queue as f64 * 0.1) + 0.5; // Simulated load calculation
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

    /// Send message to another node
    async fn send_message_to_node(&self, node_id: NodeId, message: Message) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        if let Some(address) = self.peer_addresses.get(&node_id) {
            let mut stream = TcpStream::connect(address).await?;
            let message_bytes = serde_json::to_vec(&message)?;
            stream.write_all(&message_bytes).await?;

            // Try to read response
            let mut buffer = vec![0u8; 1024 * 1024];
            match tokio::time::timeout(Duration::from_millis(500), stream.read(&mut buffer)).await {
                Ok(Ok(n)) if n > 0 => {
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
        let mut manager = self.election_manager.lock().await;

        // Collect load from all nodes
        let mut all_loads = HashMap::new();
        all_loads.insert(self.id, current_load);

        for (&peer_id, _) in &self.peer_addresses {
            let message = Message::LoadQuery { from_node: self.id };
            if let Ok(Some(Message::LoadResponse { node_id, load, .. })) =
                self.send_message_to_node(peer_id, message).await
            {
                all_loads.insert(node_id, load);
            }
        }

        // Find node with lowest load
        if let Some((&lowest_node, &lowest_load)) =
            all_loads.iter().min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        {
            let result = ElectionResult::new(lowest_node, lowest_load, all_loads);
            result.log_result();

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
