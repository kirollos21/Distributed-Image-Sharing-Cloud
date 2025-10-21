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

    if args.len() != 2 {
        eprintln!("Usage: {} <node_id>", args[0]);
        eprintln!("Example: {} 1", args[0]);
        std::process::exit(1);
    }

    let node_id: u32 = args[1].parse().expect("Invalid node ID");

    // Define node addresses (3 cloud nodes)
    let node_addresses = vec![
        (1, "127.0.0.1:8001".to_string()),
        (2, "127.0.0.1:8002".to_string()),
        (3, "127.0.0.1:8003".to_string()),
    ];

    // Get this node's address
    let my_address = node_addresses
        .iter()
        .find(|(id, _)| *id == node_id)
        .expect("Invalid node ID")
        .1
        .clone();

    // Build peer addresses map (excluding self)
    let mut peer_addresses = HashMap::new();
    for (id, addr) in node_addresses {
        if id != node_id {
            peer_addresses.insert(id, addr);
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
