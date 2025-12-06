use distributed_image_cloud::gui_client_v2::ClientAppV2;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    println!("🚀 Starting Distributed Image Cloud v2");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Distributed Image Cloud"),
        ..Default::default()
    };

    eframe::run_native(
        "Distributed Image Cloud",
        options,
        Box::new(|cc| Ok(Box::new(ClientAppV2::new(cc)))),
    )
}
