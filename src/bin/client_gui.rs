use distributed_image_cloud::gui_client::ClientApp;
use eframe::egui;
use std::env;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    // Argument 1: Client ID (optional)
    let client_id: String = if args.len() > 1 {
        args[1].clone()
    } else {
        // Generate unique client ID based on process ID and timestamp
        let pid = std::process::id();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("{}_{}", pid, timestamp % 100000)
    };

    // Argument 2+: Node addresses (optional)
    // Usage: ./client-gui <client_id> <node1_addr> <node2_addr> <node3_addr>
    // Example: ./client-gui 1 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003
    let node_addresses: Option<Vec<String>> = if args.len() > 2 {
        Some(args[2..].iter().map(|s| s.clone()).collect())
    } else {
        None
    };

    if let Some(ref addrs) = node_addresses {
        println!("Starting client GUI with ID: {}", client_id);
        println!("Connecting to nodes: {:?}", addrs);
    } else {
        println!("Starting client GUI with ID: {}", client_id);
        println!("Using default node addresses (127.0.0.1:8001-8003)");
        println!("Tip: Specify custom nodes with: ./client-gui <id> <node1> <node2> <node3>");
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        &format!("Distributed Image Cloud - Client {}", client_id),
        options,
        Box::new(move |cc| Ok(Box::new(ClientApp::new(cc, client_id.clone(), node_addresses.clone())))),
    )
}
