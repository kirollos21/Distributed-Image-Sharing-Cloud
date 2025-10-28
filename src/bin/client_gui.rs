use distributed_image_cloud::gui_client::ClientApp;
use eframe::egui;
use std::env;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    // Parse command line arguments for client ID
    let args: Vec<String> = env::args().collect();
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

    println!("Starting client GUI with ID: {}", client_id);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        &format!("Distributed Image Cloud - Client {}", client_id),
        options,
        Box::new(move |cc| Ok(Box::new(ClientApp::new(cc, client_id.clone())))),
    )
}
