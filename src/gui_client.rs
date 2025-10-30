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

    // Session state
    username: String,
    is_logged_in: bool,
    login_in_progress: Option<Promise<Result<(), String>>>,
    login_error: Option<String>,
    username_input: String,

    // Image upload state
    selected_image_path: Option<PathBuf>,
    image_preview: Option<egui::TextureHandle>,

    // Encryption parameters
    viewing_quota: u32,
    available_usernames: Vec<String>,
    selected_usernames: Vec<bool>,
    new_username_input: String,
    username_check_in_progress: Option<Promise<Result<bool, String>>>,
    username_check_error: Option<String>,

    // Request state
    current_request: Option<Promise<Result<EncryptionResult, String>>>,
    request_history: Vec<RequestHistoryItem>,

    // Send image state (after encryption)
    last_encrypted_result: Option<EncryptionResult>,
    send_image_in_progress: Option<Promise<Result<String, String>>>,

    // Received images state
    received_images: Vec<crate::messages::ReceivedImageInfo>,
    received_images_loading: Option<Promise<Result<Vec<crate::messages::ReceivedImageInfo>, String>>>,
    view_image_in_progress: Option<Promise<Result<(Vec<u8>, u32), String>>>,
    viewing_image: Option<(Vec<u8>, String, u32)>, // (image_data, image_id, remaining_views)
    viewing_image_texture: Option<egui::TextureHandle>,

    // Tokio runtime
    runtime: Option<Arc<Runtime>>,

    // UI state
    selected_tab: Tab,
    show_help: bool,
}

#[derive(PartialEq)]
enum Tab {
    Upload,
    ReceivedImages,
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
    pub fn new(_cc: &eframe::CreationContext<'_>, client_id: String) -> Self {
        // Create tokio runtime
        let runtime = Arc::new(
            tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
        );

        Self {
            client_id,
            cloud_addresses: vec![
                "10.40.98.68:8001".to_string(),
                "10.40.98.127:8002".to_string(),
                "10.40.98.225:8003".to_string(),
            ],
            runtime: Some(runtime),
            viewing_quota: 5,
            available_usernames: vec![],
            selected_usernames: vec![],
            new_username_input: String::new(),
            is_logged_in: false,
            username: String::new(),
            username_input: String::new(),
            login_in_progress: None,
            login_error: None,
            ..Default::default()
        }
    }

    fn attempt_login(&mut self) {
        let username = self.username_input.trim().to_string();

        if username.is_empty() {
            self.login_error = Some("Please enter a username".to_string());
            return;
        }

        let client_id = self.client_id.clone();
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("login", move || {
            let client = Client::new(1, cloud_addresses);
            runtime.block_on(async move {
                client.register_session(client_id, username).await
            })
        });

        self.login_in_progress = Some(promise);
        self.login_error = None;
    }

    fn render_login_screen(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);

            ui.heading(RichText::new("🖼️  Distributed Image Cloud").size(24.0));
            ui.add_space(10.0);
            ui.label(RichText::new("Welcome! Please enter your username to continue.").size(14.0));

            ui.add_space(40.0);

            ui.horizontal(|ui| {
                ui.add_space(200.0);
                ui.label(RichText::new("Username:").size(16.0));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.username_input)
                        .desired_width(250.0)
                        .hint_text("Enter your username")
                );

                // Auto-focus the text input
                if !self.is_logged_in {
                    response.request_focus();
                }

                // Handle Enter key
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.attempt_login();
                }
            });

            ui.add_space(20.0);

            // Show error message if any
            if let Some(error) = &self.login_error {
                ui.label(RichText::new(error).color(Color32::from_rgb(255, 50, 50)).size(14.0));
                ui.add_space(10.0);
            }

            // Login button or progress
            let mut should_clear_progress = false;
            if let Some(promise) = &self.login_in_progress {
                match promise.ready() {
                    None => {
                        ui.horizontal(|ui| {
                            ui.add_space(310.0);
                            ui.spinner();
                            ui.label("Logging in...");
                        });
                    }
                    Some(result) => {
                        match result {
                            Ok(()) => {
                                // Success! Set logged in state
                                self.is_logged_in = true;
                                self.username = self.username_input.clone();
                                should_clear_progress = true;
                            }
                            Err(e) => {
                                self.login_error = Some(e.clone());
                                should_clear_progress = true;
                            }
                        }
                    }
                }
            } else {
                ui.horizontal(|ui| {
                    ui.add_space(310.0);
                    if ui.button(RichText::new("Login").size(16.0)).clicked() {
                        self.attempt_login();
                    }
                });
            }

            if should_clear_progress {
                self.login_in_progress = None;
            }

            ui.add_space(20.0);
            ui.label(RichText::new("Note: Usernames must be unique across all clients").color(Color32::GRAY).size(12.0));
        });
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

                    // Show file size
                    if let Ok(metadata) = std::fs::metadata(path) {
                        let size_kb = metadata.len() / 1024;
                        let color = if size_kb > 10 {
                            Color32::from_rgb(255, 165, 0) // Orange warning
                        } else {
                            Color32::from_rgb(0, 200, 0) // Green OK
                        };
                        ui.label(RichText::new(format!("Size: {} KB", size_kb)).color(color));
                        if size_kb > 10 {
                            ui.label(RichText::new("⚠️ Large image - will be auto-compressed to ~10KB for UDP").color(Color32::from_rgb(255, 165, 0)).size(11.0));
                        }
                    }
                } else {
                    ui.label(RichText::new("No image selected").color(Color32::GRAY));
                }
            });

            ui.label(RichText::new("⚠️ UDP requires tiny images! Max 10KB (request + response must both fit in 65KB)").color(Color32::from_rgb(255, 100, 100)).size(11.0));

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

            // Viewing Quota with counter arrows
            ui.horizontal(|ui| {
                ui.label("Viewing Quota:");
                if ui.button("◀").clicked() && self.viewing_quota > 1 {
                    self.viewing_quota -= 1;
                }
                ui.label(RichText::new(format!("{}", self.viewing_quota)).strong().size(16.0));
                if ui.button("▶").clicked() && self.viewing_quota < 100 {
                    self.viewing_quota += 1;
                }
                ui.label("views");
            });

            ui.add_space(10.0);

            // Authorized Users with checkbox list
            ui.label(RichText::new("Authorized Users:").size(14.0).strong());
            ui.add_space(5.0);

            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for (i, username) in self.available_usernames.iter().enumerate() {
                        ui.checkbox(&mut self.selected_usernames[i], username);
                    }
                });

            ui.add_space(5.0);

            // Add new username section
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.new_username_input)
                    .on_hover_text("Enter a new username to add to the list");

                let can_add = !self.new_username_input.trim().is_empty()
                    && self.username_check_in_progress.is_none();

                if ui.add_enabled(can_add, egui::Button::new("➕ Add User")).clicked() {
                    let new_user = self.new_username_input.trim().to_string();

                    // Check if it's the user's own username
                    if new_user == self.username {
                        self.username_check_error = Some("Cannot add your own username".to_string());
                    } else if !self.available_usernames.contains(&new_user) {
                        self.check_username_availability(new_user);
                    } else {
                        self.username_check_error = Some("Username already in your list".to_string());
                    }
                }
            });

            // Show username check status
            if let Some(promise) = &self.username_check_in_progress {
                match promise.ready() {
                    None => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Checking if user is registered...");
                        });
                    }
                    Some(result) => {
                        match result {
                            Ok(is_available) => {
                                if !is_available {
                                    // Username is NOT available (i.e., it IS registered by someone else)
                                    // This is what we want - add them to the list
                                    let new_user = self.new_username_input.clone();
                                    self.available_usernames.push(new_user);
                                    self.selected_usernames.push(false);
                                    self.new_username_input.clear();
                                    self.username_check_error = None;
                                } else {
                                    // Username IS available (i.e., nobody is using it)
                                    // Can't send to non-existent users
                                    self.username_check_error = Some(format!("Username '{}' is not registered. Only registered users can receive images.", self.new_username_input));
                                }
                            }
                            Err(e) => {
                                self.username_check_error = Some(format!("Error checking username: {}", e));
                            }
                        }
                        self.username_check_in_progress = None;
                    }
                }
            }

            // Show error if any
            if let Some(error) = &self.username_check_error {
                ui.label(RichText::new(error).color(Color32::from_rgb(255, 100, 100)).size(11.0));
            }
        });

        ui.add_space(15.0);

        // Send request
        ui.group(|ui| {
            ui.label(RichText::new("3. Send to Cloud").size(16.0).strong());
            ui.add_space(5.0);

            let has_selected_users = self.selected_usernames.iter().any(|&selected| selected);
            let can_send = self.selected_image_path.is_some()
                && has_selected_users
                && self.current_request.is_none();

            ui.add_enabled_ui(can_send, |ui| {
                if ui.button(RichText::new("🚀 Encrypt Image").size(16.0)).clicked() {
                    self.send_encryption_request();
                }
            });

            if !can_send && self.current_request.is_none() {
                if self.selected_image_path.is_none() {
                    ui.label(RichText::new("⚠ Please select an image").color(Color32::from_rgb(255, 165, 0)));
                } else if !has_selected_users {
                    ui.label(RichText::new("⚠ Please select at least one authorized user").color(Color32::from_rgb(255, 165, 0)));
                }
            }
        });

        ui.add_space(15.0);

        // Request status
        let mut should_clear = false;
        let mut should_send_image = false;
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

                                    ui.horizontal(|ui| {
                                        if ui.button("Save Encrypted Image").clicked() {
                                            // Generate filename with same extension as original
                                            let suggested_filename = if let Some(original_path) = &self.selected_image_path {
                                                let original_name = original_path.file_stem()
                                                    .and_then(|s| s.to_str())
                                                    .unwrap_or("encrypted_image");
                                                let extension = original_path.extension()
                                                    .and_then(|s| s.to_str())
                                                    .unwrap_or("jpg");
                                                format!("{}_encrypted.{}", original_name, extension)
                                            } else {
                                                "encrypted_image.jpg".to_string()
                                            };

                                            if let Some(path) = rfd::FileDialog::new()
                                                .set_file_name(&suggested_filename)
                                                .save_file()
                                            {
                                                if let Err(e) = std::fs::write(&path, &res.encrypted_data) {
                                                    eprintln!("Failed to save: {}", e);
                                                }
                                            }
                                        }

                                        if ui.button("📤 Send to Selected Users").clicked() {
                                            self.last_encrypted_result = Some(res.clone());
                                            should_send_image = true;
                                        }
                                    });
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

        if should_send_image {
            self.send_image_to_users();
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
                ui.label(RichText::new(&self.client_id).strong());
            });

            ui.horizontal(|ui| {
                ui.label("Username:");
                ui.label(RichText::new(&self.username).strong());
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

        // Extract selected usernames from checkbox states
        let usernames: Vec<String> = self.available_usernames.iter()
            .zip(self.selected_usernames.iter())
            .filter_map(|(name, &selected)| {
                if selected {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        let quota: u32 = self.viewing_quota;
        let client_id: usize = self.client_id.parse().unwrap_or(1);
        let client_username = self.username.clone();
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
            let mut image_data = match std::fs::read(&image_path) {
                Ok(data) => data,
                Err(e) => return Err(format!("Failed to read image: {}", e)),
            };

            // UDP packet size limit is ~65KB, but we need room for:
            // - JSON serialization overhead (~30% increase)
            // - Request metadata (usernames, quota, request_id)
            // - Response needs to fit the encrypted image back
            // Limit to 10KB to ensure both request AND response fit
            const MAX_IMAGE_SIZE: usize = 10 * 1024; // 10 KB

            // Check if image is too large
            if image_data.len() > MAX_IMAGE_SIZE {
                // Try to resize/compress the image
                match image::load_from_memory(&image_data) {
                    Ok(img) => {
                        // Resize to smaller dimensions
                        let (width, height) = (img.width(), img.height());
                        let scale = (MAX_IMAGE_SIZE as f32 / image_data.len() as f32).sqrt();
                        let new_width = ((width as f32) * scale) as u32;
                        let new_height = ((height as f32) * scale) as u32;

                        let resized = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);

                        // Re-encode as JPEG with compression
                        let mut compressed = Vec::new();
                        let mut cursor = std::io::Cursor::new(&mut compressed);
                        if let Ok(_) = resized.write_to(&mut cursor, image::ImageFormat::Jpeg) {
                            if compressed.len() <= MAX_IMAGE_SIZE {
                                image_data = compressed;
                            } else {
                                return Err(format!(
                                    "Image too large for UDP! Original: {} KB, After compression: {} KB. Max allowed: {} KB.\n\nTip: Use a smaller image file, or resize it before uploading.",
                                    image_data.len() / 1024,
                                    compressed.len() / 1024,
                                    MAX_IMAGE_SIZE / 1024
                                ));
                            }
                        }
                    }
                    Err(_) => {
                        return Err(format!(
                            "Image too large for UDP! Size: {} KB, Max allowed: {} KB.\n\nThe image cannot be automatically compressed. Please:\n1. Use a smaller image file\n2. Resize the image before uploading\n3. Use a JPEG format for better compression",
                            image_data.len() / 1024,
                            MAX_IMAGE_SIZE / 1024
                        ));
                    }
                }
            }

            // Final check after compression
            eprintln!("[DEBUG] Image size after processing: {} bytes ({} KB)", image_data.len(), image_data.len() / 1024);

            // Create client and send request
            let client = Client::new(client_id, cloud_addresses);

            let result = runtime.block_on(async move {
                client
                    .send_encryption_request(request_id, client_username, image_data, usernames, quota)
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

    fn check_username_availability(&mut self, username: String) {
        let client_id = self.client_id.parse().unwrap_or(1);
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("check_username", move || {
            let client = Client::new(client_id, cloud_addresses);
            runtime.block_on(async move {
                client.check_username_available(username).await
            })
        });

        self.username_check_in_progress = Some(promise);
    }

    fn send_image_to_users(&mut self) {
        let result = match &self.last_encrypted_result {
            Some(r) => r.clone(),
            None => return,
        };

        let from_username = self.username.clone();
        let to_usernames: Vec<String> = self.available_usernames.iter()
            .zip(self.selected_usernames.iter())
            .filter_map(|(name, &selected)| if selected { Some(name.clone()) } else { None })
            .collect();

        if to_usernames.is_empty() {
            return;
        }

        let encrypted_image = result.encrypted_data.clone();
        let max_views = self.viewing_quota;
        let image_id = result.request_id.clone();
        let client_id = self.client_id.parse().unwrap_or(1);
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("send_image", move || {
            let client = Client::new(client_id, cloud_addresses);
            runtime.block_on(async move {
                client.send_image(from_username, to_usernames, encrypted_image, max_views, image_id).await
            })
        });

        self.send_image_in_progress = Some(promise);
    }

    fn load_received_images(&mut self) {
        let username = self.username.clone();
        let client_id = self.client_id.parse().unwrap_or(1);
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("load_received_images", move || {
            let client = Client::new(client_id, cloud_addresses);
            runtime.block_on(async move {
                client.query_received_images(username).await
            })
        });

        self.received_images_loading = Some(promise);
    }

    fn view_received_image(&mut self, image_id: String) {
        let username = self.username.clone();
        let client_id = self.client_id.parse().unwrap_or(1);
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("view_image", move || {
            let client = Client::new(client_id, cloud_addresses);
            runtime.block_on(async move {
                client.view_image(username, image_id).await
            })
        });

        self.view_image_in_progress = Some(promise);
    }

    fn render_received_images_tab(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.heading("📬 Received Images");
        ui.add_space(10.0);

        if ui.button("🔄 Refresh").clicked() {
            self.load_received_images();
        }

        ui.add_space(10.0);

        // Handle loading state
        if let Some(promise) = &self.received_images_loading {
            match promise.ready() {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading received images...");
                    });
                    return;
                }
                Some(result) => {
                    match result {
                        Ok(images) => {
                            self.received_images = images.clone();
                        }
                        Err(e) => {
                            ui.label(RichText::new(format!("Error loading images: {}", e))
                                .color(Color32::from_rgb(255, 0, 0)));
                        }
                    }
                    self.received_images_loading = None;
                }
            }
        }

        // Handle view image state
        if let Some(promise) = &self.view_image_in_progress {
            match promise.ready() {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading image...");
                    });
                }
                Some(result) => {
                    match result {
                        Ok((image_data, remaining_views)) => {
                            // Show debug info
                            ui.label(RichText::new("🔓 Image decrypted successfully!")
                                .color(Color32::from_rgb(0, 200, 0)).size(12.0).strong());

                            ui.label(RichText::new(format!("📊 Decrypted image: {} bytes ({:.2} KB)",
                                image_data.len(),
                                image_data.len() as f32 / 1024.0))
                                .color(Color32::GRAY).size(11.0));

                            // Check if data is empty
                            if image_data.is_empty() {
                                ui.label(RichText::new("❌ Error: Image data is empty")
                                    .color(Color32::from_rgb(255, 0, 0)));
                            } else {
                                // Try to detect format
                                let format_hint = if image_data.len() > 4 {
                                    match &image_data[0..4] {
                                        [0xFF, 0xD8, 0xFF, ..] => "JPEG",
                                        [0x89, 0x50, 0x4E, 0x47] => "PNG",
                                        _ => "Unknown",
                                    }
                                } else {
                                    "Too small"
                                };
                                ui.label(RichText::new(format!("📷 Format: {}", format_hint))
                                    .color(Color32::GRAY).size(11.0));

                                // Decode image
                                match image::load_from_memory(image_data) {
                                    Ok(img) => {
                                        ui.label(RichText::new(format!("✅ Displaying original image: {}x{}", img.width(), img.height()))
                                            .color(Color32::from_rgb(0, 200, 0)).size(11.0));

                                        let size = [img.width() as usize, img.height() as usize];
                                        let img_rgba = img.to_rgba8();
                                        let pixels = img_rgba.as_flat_samples();
                                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                                        let texture = ctx.load_texture("viewed_image", color_image, Default::default());

                                        self.viewing_image_texture = Some(texture);
                                        self.viewing_image = Some((image_data.clone(), String::new(), *remaining_views));
                                    }
                                    Err(e) => {
                                        ui.label(RichText::new(format!("❌ Failed to display image: {}", e))
                                            .color(Color32::from_rgb(255, 0, 0)));
                                        ui.label(RichText::new(format!("First 10 bytes: {:?}", &image_data[..image_data.len().min(10)]))
                                            .color(Color32::GRAY).size(10.0));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            ui.label(RichText::new(format!("❌ Error fetching image: {}", e))
                                .color(Color32::from_rgb(255, 0, 0)));
                        }
                    }
                    self.view_image_in_progress = None;
                }
            }
        }

        // Show received images list
        if self.received_images.is_empty() {
            ui.label(RichText::new("No images received").color(Color32::GRAY));
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for img_info in &self.received_images.clone() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(format!("From: {}", img_info.from_username)).strong());
                                ui.label(format!("ID: {}", img_info.image_id));
                                ui.label(format!("Remaining views: {}", img_info.remaining_views));
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("👁 View").clicked() {
                                    self.view_received_image(img_info.image_id.clone());
                                }
                            });
                        });
                    });
                    ui.add_space(5.0);
                }
            });
        }

        // Show viewing image
        if let Some(texture) = &self.viewing_image_texture {
            ui.add_space(10.0);
            ui.separator();
            ui.heading("Viewing Image");

            if let Some((_, _, remaining)) = &self.viewing_image {
                ui.label(format!("Remaining views: {}", remaining));
            }

            let max_size = 400.0;
            let size = texture.size_vec2();
            let scale = (max_size / size.x).min(max_size / size.y).min(1.0);
            ui.image((texture.id(), size * scale));

            if ui.button("Close").clicked() {
                self.viewing_image = None;
                self.viewing_image_texture = None;
                self.load_received_images(); // Reload to update counters
            }
        }
    }
}

impl eframe::App for ClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Repaint continuously to update async operations
        ctx.request_repaint();

        // Show login screen if not logged in
        if !self.is_logged_in {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.render_login_screen(ui);
            });
            return;
        }

        // Main UI (only shown when logged in)
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🖼️  Distributed Image Cloud - Client");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("❓ Help").clicked() {
                        self.show_help = !self.show_help;
                    }

                    ui.separator();

                    ui.label(RichText::new(format!("👤 {}", self.username))
                        .color(Color32::from_rgb(0, 200, 255))
                        .strong());
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
                ui.selectable_value(&mut self.selected_tab, Tab::ReceivedImages, "📬 Received");
                ui.selectable_value(&mut self.selected_tab, Tab::History, "📜 History");
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "⚙️ Settings");
            });

            ui.separator();
            ui.add_space(10.0);

            match self.selected_tab {
                Tab::Upload => self.render_upload_tab(ui, ctx),
                Tab::ReceivedImages => self.render_received_images_tab(ui, ctx),
                Tab::History => self.render_history_tab(ui),
                Tab::Settings => self.render_settings_tab(ui),
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Unregister session when app closes
        if self.is_logged_in && !self.username.is_empty() {
            let client_id = self.client_id.clone();
            let username = self.username.clone();
            let cloud_addresses = self.cloud_addresses.clone();
            let runtime = self.runtime.as_ref().unwrap().clone();

            runtime.block_on(async move {
                let client = Client::new(1, cloud_addresses);
                client.unregister_session(client_id, username).await;
            });
        }
    }
}
