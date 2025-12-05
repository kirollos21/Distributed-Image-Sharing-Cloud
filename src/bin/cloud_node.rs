use distributed_image_cloud::node::CloudNode;
use distributed_image_cloud::firebase::{FireBaseClient, NodeStatus};
use env_logger::Env;
use log::{info, warn, error};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

/// Get local IP address for binding
fn get_local_ip() -> String {
    use std::net::UdpSocket;
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    return addr.ip().to_string();
                }
            }
            "127.0.0.1".to_string()
        }
        Err(_) => "127.0.0.1".to_string(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <node_id> [bind_address]", args[0]);
        eprintln!("Example: {} 1", args[0]);
        eprintln!("Example: {} 1 192.168.1.10:8001", args[0]);
        eprintln!("");
        eprintln!("If bind_address is not provided, it will use local IP with port 800<node_id>");
        std::process::exit(1);
    }

    let node_id: u8 = args[1].parse().expect("Invalid node ID (must be 1-255)");
    
    // Determine bind address
    let my_address = if args.len() > 2 {
        args[2].clone()
    } else {
        // Auto-detect: use local IP with port 800X
        let local_ip = get_local_ip();
        format!("{}:800{}", local_ip, node_id)
    };

    info!("Starting Cloud Node {}", node_id);
    info!("Address: {}", my_address);

    // Fetch peer addresses from Firebase
    let firebase = FireBaseClient::new();
    let mut peer_addresses = HashMap::new();
    
    match firebase.get_all_nodes().await {
        Ok(nodes) => {
            for (node_name, node_info) in nodes {
                // Parse node ID from name (e.g., "node1" -> 1)
                if let Some(id_str) = node_name.strip_prefix("node") {
                    if let Ok(peer_id) = id_str.parse::<u8>() {
                        if peer_id != node_id {
                            peer_addresses.insert(peer_id, node_info.address.clone());
                            info!("Found peer node {}: {}", peer_id, node_info.address);
                        }
                    }
                }
            }
        }
        Err(e) => {
            warn!("Failed to fetch nodes from Firebase: {}. Starting with no peers.", e);
        }
    }

    info!("Peers: {:?}", peer_addresses);

    // Create and start the node
    let node = Arc::new(CloudNode::new(node_id, my_address, peer_addresses));

    node.start().await?;

    Ok(())
}
