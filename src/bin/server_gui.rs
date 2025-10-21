use distributed_image_cloud::gui_server::ServerMonitorApp;
use distributed_image_cloud::node::CloudNode;
use eframe::egui;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    // Parse command line arguments for optional node connection
    let args: Vec<String> = env::args().collect();
    let node_id: Option<u32> = if args.len() > 1 {
        args[1].parse().ok()
    } else {
        None
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 800.0])
            .with_min_inner_size([900.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Distributed Image Cloud - Server Monitor",
        options,
        Box::new(move |cc| {
            let app = if let Some(id) = node_id {
                // Connect to a specific node
                let node_addresses = vec![
                    (1, "127.0.0.1:9001".to_string()),  // Using different ports for GUI monitoring
                    (2, "127.0.0.1:9002".to_string()),
                    (3, "127.0.0.1:9003".to_string()),
                ];

                let my_address = node_addresses
                    .iter()
                    .find(|(nid, _)| *nid == id)
                    .map(|(_, addr)| addr.clone())
                    .unwrap_or_else(|| format!("127.0.0.1:900{}", id));

                let mut peer_addresses = HashMap::new();
                for (peer_id, addr) in node_addresses {
                    if peer_id != id {
                        peer_addresses.insert(peer_id, addr);
                    }
                }

                let node = Arc::new(CloudNode::new(id, my_address, peer_addresses));

                // Start node in background
                let node_clone = node.clone();
                tokio::spawn(async move {
                    if let Err(e) = node_clone.start().await {
                        eprintln!("Node error: {}", e);
                    }
                });

                ServerMonitorApp::new(cc).with_node(node)
            } else {
                // Standalone monitoring mode
                ServerMonitorApp::new(cc)
            };

            Ok(Box::new(app))
        }),
    )
}
