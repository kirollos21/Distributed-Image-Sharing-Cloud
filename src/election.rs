use crate::messages::{Message, NodeId};
use log::info;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

/// Bully election algorithm implementation with load-based selection
///
/// Modified Bully algorithm where the node with the LOWEST load wins
/// instead of the highest ID. This provides transparent load balancing.
pub struct ElectionManager {
    pub node_id: NodeId,
    pub node_addresses: HashMap<NodeId, String>,
    pub current_coordinator: Option<NodeId>,
}

impl ElectionManager {
    pub fn new(node_id: NodeId, node_addresses: HashMap<NodeId, String>) -> Self {
        Self {
            node_id,
            node_addresses,
            current_coordinator: None,
        }
    }

    /// Initiate an election based on current load
    /// Returns the elected coordinator's ID
    pub async fn start_election(
        &mut self,
        current_load: f64,
        send_message: impl Fn(NodeId, Message) -> bool,
    ) -> Option<NodeId> {
        info!(
            "[Node {}] Starting election with load: {:.2}",
            self.node_id, current_load
        );

        // Query all other nodes for their load
        let mut node_loads = HashMap::new();
        node_loads.insert(self.node_id, current_load);

        // Send ELECTION message to all other nodes
        for (&other_node, _) in &self.node_addresses {
            if other_node != self.node_id {
                send_message(other_node, Message::Election {
                    from_node: self.node_id,
                });
            }
        }

        // Wait a bit for responses
        sleep(Duration::from_millis(100)).await;

        // Send load queries to all nodes
        for (&other_node, _) in &self.node_addresses {
            if other_node != self.node_id {
                send_message(other_node, Message::LoadQuery {
                    from_node: self.node_id,
                });
            }
        }

        // In a real implementation, we would collect responses here
        // For this simulation, we assume the lowest-load node wins

        node_loads
            .iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(id, _)| *id)
    }

    /// Handle incoming election message
    pub fn handle_election_message(
        &self,
        from_node: NodeId,
        my_load: f64,
        send_message: impl Fn(NodeId, Message) -> bool,
    ) {
        info!(
            "[Node {}] Received ELECTION from Node {}",
            self.node_id, from_node
        );

        // Always respond with OK to let the initiator know we're alive
        send_message(from_node, Message::Ok {
            from_node: self.node_id,
        });

        // Also send our load
        send_message(from_node, Message::LoadResponse {
            node_id: self.node_id,
            load: my_load,
            queue_length: 0, // Will be filled by actual node
            processed_count: 0, // Will be filled by actual node
        });
    }

    /// Announce this node as coordinator
    pub fn announce_coordinator(
        &mut self,
        load: f64,
        send_message: impl Fn(NodeId, Message) -> bool,
    ) {
        info!(
            "[Node {}] Announcing as COORDINATOR with load: {:.2}",
            self.node_id, load
        );

        self.current_coordinator = Some(self.node_id);

        // Broadcast coordinator message to all nodes
        for (&other_node, _) in &self.node_addresses {
            if other_node != self.node_id {
                send_message(other_node, Message::Coordinator {
                    node_id: self.node_id,
                    load,
                });
            }
        }
    }

    /// Update the current coordinator
    pub fn update_coordinator(&mut self, coordinator_id: NodeId, load: f64) {
        if self.current_coordinator != Some(coordinator_id) {
            info!(
                "[Node {}] New COORDINATOR: Node {} (load: {:.2})",
                self.node_id, coordinator_id, load
            );
            self.current_coordinator = Some(coordinator_id);
        }
    }

    /// Check if this node is the coordinator
    pub fn is_coordinator(&self) -> bool {
        self.current_coordinator == Some(self.node_id)
    }

    /// Get the current coordinator
    pub fn get_coordinator(&self) -> Option<NodeId> {
        self.current_coordinator
    }
}

/// Election result with load information
#[derive(Debug, Clone)]
pub struct ElectionResult {
    pub coordinator_id: NodeId,
    pub load: f64,
    pub all_loads: HashMap<NodeId, f64>,
}

impl ElectionResult {
    pub fn new(coordinator_id: NodeId, load: f64, all_loads: HashMap<NodeId, f64>) -> Self {
        Self {
            coordinator_id,
            load,
            all_loads,
        }
    }

    pub fn log_result(&self) {
        info!("=== ELECTION RESULT ===");
        info!("Coordinator: Node {} (load: {:.2})", self.coordinator_id, self.load);
        info!("All node loads:");
        let mut sorted_loads: Vec<_> = self.all_loads.iter().collect();
        sorted_loads.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap());
        for (node_id, load) in sorted_loads {
            let is_coord = if *node_id == self.coordinator_id { " [COORDINATOR]" } else { "" };
            info!("  Node {}: {:.2}{}", node_id, load, is_coord);
        }
        info!("=======================");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_election_manager_creation() {
        let mut addresses = HashMap::new();
        addresses.insert(1, "127.0.0.1:8001".to_string());
        addresses.insert(2, "127.0.0.1:8002".to_string());

        let manager = ElectionManager::new(1, addresses);
        assert_eq!(manager.node_id, 1);
        assert_eq!(manager.current_coordinator, None);
    }

    #[test]
    fn test_coordinator_announcement() {
        let mut addresses = HashMap::new();
        addresses.insert(1, "127.0.0.1:8001".to_string());

        let mut manager = ElectionManager::new(1, addresses);

        let send_fn = |_node: NodeId, _msg: Message| true;
        manager.announce_coordinator(0.5, send_fn);

        assert_eq!(manager.current_coordinator, Some(1));
        assert!(manager.is_coordinator());
    }
}
