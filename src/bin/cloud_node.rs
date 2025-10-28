use distributed_image_cloud::node::CloudNode;
use env_logger::Env;
use log::info;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <node_id> <bind_address> <peer_addresses>", args[0]);
        eprintln!("Example (local):      {} 1 127.0.0.1:8001 127.0.0.1:8002,127.0.0.1:8003", args[0]);
        eprintln!("Example (multi-device): {} 1 0.0.0.0:8001 192.168.1.11:8002,192.168.1.12:8003", args[0]);
        std::process::exit(1);
    }

    let node_id: u32 = args[1].parse().expect("Invalid node ID");
    let my_address = args[2].clone();

    // Parse peer addresses from comma-separated list
    let mut peer_addresses = HashMap::new();
    if args.len() > 3 {
        let peers_str = &args[3];
        for (idx, peer_addr) in peers_str.split(',').enumerate() {
            // Assign peer IDs based on position (not ideal but works)
            // For 3 nodes: if we're node 1, peers are 2,3; if node 2, peers are 1,3; if node 3, peers are 1,2
            let mut peer_id = idx as u32 + 1;
            if peer_id >= node_id {
                peer_id += 1;
            }
            peer_addresses.insert(peer_id, peer_addr.trim().to_string());
        }
    }

    info!("Starting Cloud Node {}", node_id);
    info!("Address: {}", my_address);
    info!("Peers: {:?}", peer_addresses);

    // Create and start the node
    let node = Arc::new(CloudNode::new(node_id, my_address, peer_addresses));

    node.start().await?;

    Ok(())
}
