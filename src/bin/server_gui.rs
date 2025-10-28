use distributed_image_cloud::gui_server::ServerMonitorApp;
use eframe::egui;
use std::env;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    // Parse command line arguments for optional node ID (for display only)
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
            // Create app in standalone monitoring mode
            // The GUI monitors external nodes that are already running
            let mut app = ServerMonitorApp::new(cc);

            if let Some(id) = node_id {
                // Set the node ID for display purposes
                app.set_monitored_node_id(id);
            }

            Ok(Box::new(app))
        }),
    )
}
