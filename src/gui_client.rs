use crate::client::Client;
use crate::messages::Message;
use eframe::egui;
use egui::{Color32, RichText, Ui};
use poll_promise::Promise;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Default)]
pub struct ClientApp {
    // Client configuration
    client_id: String,
    cloud_addresses: Vec<String>,

    // Image upload state
    selected_image_path: Option<PathBuf>,
    image_preview: Option<egui::TextureHandle>,

    // Encryption parameters
    authorized_users: String,
    viewing_quota: String,

    // Request state
    current_request: Option<Promise<Result<EncryptionResult, String>>>,
    request_history: Vec<RequestHistoryItem>,

    // Tokio runtime
    runtime: Option<Arc<Runtime>>,

    // UI state
    selected_tab: Tab,
    show_help: bool,
}

#[derive(PartialEq)]
enum Tab {
    Upload,
    History,
    Settings,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Upload
    }
}

#[derive(Clone)]
struct EncryptionResult {
    success: bool,
    request_id: String,
    encrypted_data: Vec<u8>,
    error: Option<String>,
    duration_ms: u64,
}

#[derive(Clone)]
struct RequestHistoryItem {
    request_id: String,
    timestamp: String,
    success: bool,
    duration_ms: u64,
    image_path: String,
    users_count: usize,
}

impl ClientApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Create tokio runtime
        let runtime = Arc::new(
            tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
        );

        Self {
            client_id: "1".to_string(),
            cloud_addresses: vec![
                "127.0.0.1:8001".to_string(),
                "127.0.0.1:8002".to_string(),
                "127.0.0.1:8003".to_string(),
            ],
            runtime: Some(runtime),
            authorized_users: "alice, bob, charlie".to_string(),
            viewing_quota: "5".to_string(),
            ..Default::default()
        }
    }

    fn render_upload_tab(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.heading("📤 Upload & Encrypt Image");
        ui.add_space(10.0);

        // Image selection
        ui.group(|ui| {
            ui.label(RichText::new("1. Select Image").size(16.0).strong());
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("📂 Choose Image File").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                        .pick_file()
                    {
                        self.selected_image_path = Some(path.clone());

                        // Load image preview
                        if let Ok(img) = image::open(&path) {
                            let size = [img.width() as usize, img.height() as usize];
                            let img_rgba = img.to_rgba8();
                            let pixels = img_rgba.as_flat_samples();

                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                size,
                                pixels.as_slice(),
                            );

                            self.image_preview = Some(ctx.load_texture(
                                "preview",
                                color_image,
                                Default::default(),
                            ));
                        }
                    }
                }

                if let Some(path) = &self.selected_image_path {
                    ui.label(format!("Selected: {}", path.display()));
                } else {
                    ui.label(RichText::new("No image selected").color(Color32::GRAY));
                }
            });

            // Show image preview
            if let Some(texture) = &self.image_preview {
                ui.add_space(10.0);
                ui.label("Preview:");
                let max_size = 300.0;
                let size = texture.size_vec2();
                let scale = (max_size / size.x).min(max_size / size.y).min(1.0);
                ui.image((texture.id(), size * scale));
            }
        });

        ui.add_space(15.0);

        // Encryption parameters
        ui.group(|ui| {
            ui.label(RichText::new("2. Configure Encryption").size(16.0).strong());
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Authorized Users:");
                ui.text_edit_singleline(&mut self.authorized_users);
            });
            ui.label(RichText::new("(Comma-separated usernames)").color(Color32::GRAY).size(12.0));

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Viewing Quota:    ");
                ui.add(egui::TextEdit::singleline(&mut self.viewing_quota).desired_width(80.0));
                ui.label("views");
            });
        });

        ui.add_space(15.0);

        // Send request
        ui.group(|ui| {
            ui.label(RichText::new("3. Send to Cloud").size(16.0).strong());
            ui.add_space(5.0);

            let can_send = self.selected_image_path.is_some()
                && !self.authorized_users.trim().is_empty()
                && self.viewing_quota.parse::<u32>().is_ok()
                && self.current_request.is_none();

            ui.add_enabled_ui(can_send, |ui| {
                if ui.button(RichText::new("🚀 Encrypt Image").size(16.0)).clicked() {
                    self.send_encryption_request();
                }
            });

            if !can_send && self.current_request.is_none() {
                if self.selected_image_path.is_none() {
                    ui.label(RichText::new("⚠ Please select an image").color(Color32::from_rgb(255, 165, 0)));
                } else if self.authorized_users.trim().is_empty() {
                    ui.label(RichText::new("⚠ Please add authorized users").color(Color32::from_rgb(255, 165, 0)));
                } else if self.viewing_quota.parse::<u32>().is_err() {
                    ui.label(RichText::new("⚠ Invalid quota (must be a number)").color(Color32::from_rgb(255, 165, 0)));
                }
            }
        });

        ui.add_space(15.0);

        // Request status
        let mut should_clear = false;
        if let Some(promise) = &self.current_request {
            ui.group(|ui| {
                ui.label(RichText::new("Request Status").size(16.0).strong());
                ui.add_space(5.0);

                match promise.ready() {
                    None => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Processing encryption request...");
                        });
                    }
                    Some(result) => {
                        match result {
                            Ok(res) => {
                                if res.success {
                                    ui.label(RichText::new(format!("✅ Success! Request ID: {}", res.request_id))
                                        .color(Color32::from_rgb(0, 200, 0))
                                        .size(14.0));
                                    ui.label(format!("Duration: {}ms", res.duration_ms));
                                    ui.label(format!("Encrypted data size: {} bytes", res.encrypted_data.len()));

                                    if ui.button("Save Encrypted Image").clicked() {
                                        if let Some(path) = rfd::FileDialog::new()
                                            .set_file_name("encrypted_image.dat")
                                            .save_file()
                                        {
                                            if let Err(e) = std::fs::write(&path, &res.encrypted_data) {
                                                eprintln!("Failed to save: {}", e);
                                            }
                                        }
                                    }
                                } else {
                                    ui.label(RichText::new(format!("❌ Failed: {}", res.error.as_deref().unwrap_or("Unknown error")))
                                        .color(Color32::from_rgb(255, 0, 0))
                                        .size(14.0));
                                }

                                if ui.button("Clear").clicked() {
                                    should_clear = true;
                                }
                            }
                            Err(e) => {
                                ui.label(RichText::new(format!("❌ Error: {}", e))
                                    .color(Color32::from_rgb(255, 0, 0))
                                    .size(14.0));

                                if ui.button("Clear").clicked() {
                                    should_clear = true;
                                }
                            }
                        }
                    }
                }
            });
        }

        if should_clear {
            self.current_request = None;
        }
    }

    fn render_history_tab(&mut self, ui: &mut Ui) {
        ui.heading("📜 Request History");
        ui.add_space(10.0);

        if self.request_history.is_empty() {
            ui.label(RichText::new("No requests yet").color(Color32::GRAY).size(14.0));
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (_i, item) in self.request_history.iter().enumerate().rev() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let status_text = if item.success { "✅" } else { "❌" };
                        ui.label(RichText::new(status_text).size(20.0));

                        ui.vertical(|ui| {
                            ui.label(RichText::new(&item.request_id).strong());
                            ui.label(RichText::new(&item.timestamp).size(12.0).color(Color32::GRAY));
                        });
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label(format!("📁 {}", item.image_path));
                        ui.label(format!("👥 {} users", item.users_count));
                        ui.label(format!("⏱ {}ms", item.duration_ms));
                    });
                });

                ui.add_space(5.0);
            }
        });
    }

    fn render_settings_tab(&mut self, ui: &mut Ui) {
        ui.heading("⚙️ Settings");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Client Configuration").size(16.0).strong());
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Client ID:");
                ui.text_edit_singleline(&mut self.client_id);
            });
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("Cloud Nodes").size(16.0).strong());
            ui.add_space(5.0);

            let mut to_remove = None;
            for (i, addr) in self.cloud_addresses.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Node {}:", i + 1));
                    ui.text_edit_singleline(addr);
                    if ui.button("❌").clicked() {
                        to_remove = Some(i);
                    }
                });
            }

            if let Some(i) = to_remove {
                self.cloud_addresses.remove(i);
            }

            ui.add_space(5.0);
            if ui.button("➕ Add Node").clicked() {
                self.cloud_addresses.push(format!("127.0.0.1:{}", 8000 + self.cloud_addresses.len() + 1));
            }
        });
    }

    fn send_encryption_request(&mut self) {
        let image_path = self.selected_image_path.as_ref().unwrap().clone();
        let usernames: Vec<String> = self
            .authorized_users
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let quota: u32 = self.viewing_quota.parse().unwrap();
        let client_id: usize = self.client_id.parse().unwrap_or(1);
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        // Add to history
        let request_id = format!("client_{}_req_{}", client_id, chrono::Utc::now().timestamp());
        self.request_history.push(RequestHistoryItem {
            request_id: request_id.clone(),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            success: false,
            duration_ms: 0,
            image_path: image_path.display().to_string(),
            users_count: usernames.len(),
        });

        // Create promise for async request
        let promise = Promise::spawn_thread("encryption_request", move || {
            let start = std::time::Instant::now();

            // Read image file
            let image_data = match std::fs::read(&image_path) {
                Ok(data) => data,
                Err(e) => return Err(format!("Failed to read image: {}", e)),
            };

            // Create client and send request
            let client = Client::new(client_id, cloud_addresses);

            let result = runtime.block_on(async move {
                client
                    .send_encryption_request(request_id, image_data, usernames, quota)
                    .await
            });

            let duration = start.elapsed().as_millis() as u64;

            match result {
                Ok(Message::EncryptionResponse {
                    request_id,
                    encrypted_image,
                    success,
                    error,
                }) => Ok(EncryptionResult {
                    success,
                    request_id,
                    encrypted_data: encrypted_image,
                    error,
                    duration_ms: duration,
                }),
                Ok(_) => Err("Unexpected response type".to_string()),
                Err(e) => Err(format!("Request failed: {}", e)),
            }
        });

        self.current_request = Some(promise);
    }
}

impl eframe::App for ClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Repaint continuously to update async operations
        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🖼️  Distributed Image Cloud - Client");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("❓ Help").clicked() {
                        self.show_help = !self.show_help;
                    }
                });
            });
        });

        if self.show_help {
            egui::Window::new("Help")
                .open(&mut self.show_help)
                .show(ctx, |ui| {
                    ui.label("How to use:");
                    ui.label("1. Select an image file to encrypt");
                    ui.label("2. Enter comma-separated usernames who can view the image");
                    ui.label("3. Set how many times the image can be viewed");
                    ui.label("4. Click 'Encrypt Image' to send to the cloud");
                    ui.label("5. Save the encrypted image when ready");
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, Tab::Upload, "📤 Upload");
                ui.selectable_value(&mut self.selected_tab, Tab::History, "📜 History");
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "⚙️ Settings");
            });

            ui.separator();
            ui.add_space(10.0);

            match self.selected_tab {
                Tab::Upload => self.render_upload_tab(ui, ctx),
                Tab::History => self.render_history_tab(ui),
                Tab::Settings => self.render_settings_tab(ui),
            }
        });
    }
}
