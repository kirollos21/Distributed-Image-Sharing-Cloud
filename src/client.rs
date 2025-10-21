use crate::messages::Message;
use crate::metrics::MetricsCollector;
use log::{debug, error, info, warn};
use rand::Rng;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
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

    /// Send an encryption request by multicasting to all cloud nodes
    /// Returns the first successful response
    pub async fn send_encryption_request(
        &self,
        request_id: String,
        image_data: Vec<u8>,
        usernames: Vec<String>,
        quota: u32,
    ) -> Result<Message, String> {
        let message = Message::EncryptionRequest {
            request_id: request_id.clone(),
            image_data,
            usernames,
            quota,
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
        // Try to connect
        let mut stream = match TcpStream::connect(address).await {
            Ok(s) => s,
            Err(e) => {
                warn!("[Client {}] Failed to connect to {}: {}", client_id, address, e);
                return Err(format!("Connection failed: {}", e));
            }
        };

        // Send message
        let message_bytes = serde_json::to_vec(&message).map_err(|e| e.to_string())?;
        stream
            .write_all(&message_bytes)
            .await
            .map_err(|e| e.to_string())?;

        // Read response with timeout
        let mut buffer = vec![0u8; 2 * 1024 * 1024]; // 2MB buffer for response
        let n = match tokio::time::timeout(Duration::from_secs(10), stream.read(&mut buffer)).await
        {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => {
                return Err(format!("Read error: {}", e));
            }
            Err(_) => {
                return Err("Timeout waiting for response".to_string());
            }
        };

        if n == 0 {
            return Err("Empty response".to_string());
        }

        // Parse response
        let response: Message = serde_json::from_slice(&buffer[..n]).map_err(|e| e.to_string())?;

        Ok(response)
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
        let usernames = vec![
            format!("user_{}", self.id),
            format!("user_{}", (self.id + 1) % 100),
        ];
        let quota = 5;

        match self
            .send_encryption_request(request_id.clone(), image_data, usernames, quota)
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
