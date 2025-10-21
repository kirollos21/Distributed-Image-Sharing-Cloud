use distributed_image_cloud::gui_client::ClientApp;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Distributed Image Cloud - Client",
        options,
        Box::new(|cc| Ok(Box::new(ClientApp::new(cc)))),
    )
}
