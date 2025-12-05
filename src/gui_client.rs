use crate::client::Client;
use crate::firebase::{FireBaseClient, UserInfo, UserStatus};
use crate::messages::Message;
use eframe::egui;
use egui::{Color32, RichText, Ui, ColorImage, TextureHandle};
use poll_promise::Promise;
use std::path::PathBuf;
use std::sync::Arc;
use std::net::UdpSocket;
use tokio::runtime::Runtime;

/// Get the local IP address by connecting to a remote address
fn get_local_ip() -> String {
    // Connect to a public DNS to determine local IP
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => {
            // Connect to Google's DNS (doesn't actually send data)
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    return addr.ip().to_string();
                }
            }
            "127.0.0.1".to_string()
        }
        Err(_) => "127.0.0.1".to_string(),
    }
}

/// Decode a base64 data URL to a ColorImage
fn decode_base64_image(data_url: &str) -> Option<ColorImage> {
    // Extract base64 data from data URL (format: "data:image/png;base64,...")
    let base64_data = if data_url.contains(",") {
        data_url.split(',').nth(1)?
    } else {
        data_url
    };
    
    // Decode base64
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data).ok()?;
    
    // Load image
    let img = image::load_from_memory(&bytes).ok()?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    
    Some(ColorImage::from_rgba_unmultiplied(size, &pixels))
}

#[derive(Default)]
pub struct ClientApp {
    // Client configuration
    client_id: String,
    cloud_addresses: Vec<String>,
    firebase_client: FireBaseClient,

    // Session state
    username: String,
    user_id: String,  // Firebase user ID
    is_logged_in: bool,
    login_in_progress: Option<Promise<Result<(String, String), String>>>,  // Returns (username, user_id) on success
    login_error: Option<String>,
    username_input: String,
    password_input: String,  // password field
    is_registering: bool,    // toggle between login and register
    just_registered_id: Option<String>,  // Show ID after registration

    // Image upload state
    selected_image_path: Option<PathBuf>,
    image_preview: Option<egui::TextureHandle>,

    // Encryption parameters
    viewing_quota: u8,
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
    view_image_in_progress: Option<Promise<Result<(Vec<u8>, u8), String>>>,
    viewing_image: Option<(Vec<u8>, String, u8)>, // (image_data, image_id, remaining_views)
    viewing_image_texture: Option<egui::TextureHandle>,

    // My Gallery state
    my_gallery: Vec<String>,  // URLs/paths of gallery images
    my_gallery_textures: Vec<Option<egui::TextureHandle>>,
    gallery_loading: Option<Promise<Result<Vec<String>, String>>>,
    gallery_upload_in_progress: Option<Promise<Result<(), String>>>,
    gallery_error: Option<String>,
    gallery_loaded: bool,  // Track if gallery has been loaded

    // Browse Users state
    user_search_input: String,
    searched_user: Option<crate::firebase::UserInfo>,
    searched_user_id: Option<String>,
    user_search_in_progress: Option<Promise<Result<Option<(String, crate::firebase::UserInfo)>, String>>>,
    user_search_error: Option<String>,
    viewed_gallery: Vec<String>,
    viewed_gallery_textures: Vec<Option<egui::TextureHandle>>,

    // Tokio runtime
    runtime: Option<Arc<Runtime>>,

    // Heartbeat for presence tracking (not using Default, initialized in new())
    #[allow(dead_code)]
    last_heartbeat: Option<std::time::Instant>,

    // UI state
    selected_tab: Tab,
    show_help: bool,
}

#[derive(PartialEq)]
enum Tab {
    Upload,
    ReceivedImages,
    MyGallery,
    BrowseUsers,
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
    pub fn new(_cc: &eframe::CreationContext<'_>, client_id: String, node_addresses: Option<Vec<String>>) -> Self {
        // Create tokio runtime
        let runtime = Arc::new(
            tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
        );

        // Use provided addresses or default to localhost
        let cloud_addresses = node_addresses.unwrap_or_else(|| vec![
            "127.0.0.1:8001".to_string(),
            "127.0.0.1:8002".to_string(),
            "127.0.0.1:8003".to_string(),
        ]);

        println!("Client will connect to nodes: {:?}", cloud_addresses);

        Self {
            client_id,
            cloud_addresses,
            firebase_client: FireBaseClient::new(),
            runtime: Some(runtime),
            viewing_quota: 5,
            available_usernames: vec![],
            selected_usernames: vec![],
            new_username_input: String::new(),
            is_logged_in: false,
            username: String::new(),
            user_id: String::new(),
            username_input: String::new(),
            password_input: String::new(),
            is_registering: false,
            login_in_progress: None,
            login_error: None,
            last_heartbeat: Some(std::time::Instant::now()),
            ..Default::default()
        }
    }

    fn send_heartbeat(&self) {
        let user_id = self.user_id.clone();
        if user_id.is_empty() {
            return;
        }
        
        let runtime = self.runtime.as_ref().unwrap().clone();
        std::thread::spawn(move || {
            let firebase = FireBaseClient::new();
            runtime.block_on(async move {
                let now = chrono::Utc::now().timestamp();
                let _ = firebase.update_last_seen(&user_id, now).await;
            });
        });
    }

    fn logout(&mut self) {
        // Update status to Offline in Firebase
        let user_id = self.user_id.clone();
        if !user_id.is_empty() {
            let runtime = self.runtime.as_ref().unwrap().clone();
            std::thread::spawn(move || {
                let firebase = FireBaseClient::new();
                runtime.block_on(async move {
                    let _ = firebase.update_user_status(&user_id, &crate::firebase::UserStatus::Offline).await;
                });
            });
        }

        // Reset session state
        self.is_logged_in = false;
        self.username.clear();
        self.user_id.clear();
        self.username_input.clear();
        self.password_input.clear();
        self.login_error = None;
        self.is_registering = false;

        // Reset gallery state
        self.my_gallery.clear();
        self.my_gallery_textures.clear();
        self.gallery_loaded = false;
        self.gallery_error = None;
        self.gallery_loading = None;

        // Reset browse users state
        self.user_search_input.clear();
        self.searched_user = None;
        self.searched_user_id = None;
        self.viewed_gallery.clear();
        self.viewed_gallery_textures.clear();

        // Reset other state
        self.selected_image_path = None;
        self.image_preview = None;
        self.selected_tab = Tab::Upload;
    }

    fn attempt_login(&mut self) {
        let input = self.username_input.trim().to_string();
        let password = self.password_input.clone();
        let is_registering = self.is_registering;

        if input.is_empty() {
            if is_registering {
                self.login_error = Some("Please enter a username".to_string());
            } else {
                self.login_error = Some("Please enter username#id (e.g., potato#1)".to_string());
            }
            return;
        }

        if password.is_empty() {
            self.login_error = Some("Please enter a password".to_string());
            return;
        }

        let (username, user_id) = if is_registering {
            // For registration, just use username - ID will be auto-generated
            (input.clone(), String::new())
        } else {
            // For login, parse username#id format
            let parts: Vec<&str> = input.split('#').collect();
            if parts.len() != 2 {
                self.login_error = Some("Invalid format. Use username#id (e.g., potato#1)".to_string());
                return;
            }
            let username = parts[0].to_string();
            let user_id = parts[1].to_string();

            if username.is_empty() || user_id.is_empty() {
                self.login_error = Some("Both username and ID are required (e.g., potato#1)".to_string());
                return;
            }
            (username, user_id)
        };

        let firebase = FireBaseClient::new();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("login", move || {
            runtime.block_on(async move {
                if is_registering {
                    // REGISTER: Generate unique ID and create user (allow duplicate usernames)
                    // Try to find a unique ID (up to 10 attempts)
                    let mut generated_id = String::new();
                    let mut attempts = 0;
                    loop {
                        let candidate_id = format!("{}", (chrono::Utc::now().timestamp_millis() % 99) + 1);
                        match firebase.get_user(&candidate_id).await {
                            Ok(None) => {
                                // ID is available
                                generated_id = candidate_id;
                                break;
                            }
                            Ok(Some(_)) => {
                                // ID exists, try again with slight delay
                                attempts += 1;
                                if attempts >= 10 {
                                    return Err("Could not generate unique ID. Please try again.".to_string());
                                }
                                std::thread::sleep(std::time::Duration::from_millis(10));
                            }
                            Err(e) => return Err(format!("Network error: {}", e)),
                        }
                    }
                    
                    let local_ip = get_local_ip();
                    let new_user = UserInfo {
                        id: generated_id.clone(),
                        username: username.clone(),
                        password: password,
                        status: UserStatus::Online,
                        last_seen: chrono::Utc::now().timestamp(),
                        ip: local_ip,
                        gallery: vec![],
                    };
                    firebase.create_user(&new_user).await
                        .map_err(|e| format!("Failed to create user: {}", e))?;
                    Ok((username, generated_id))
                } else {
                    // LOGIN: Get user directly by ID and verify
                    match firebase.get_user(&user_id).await {
                        Ok(Some(user_info)) => {
                            // Check username matches
                            if user_info.username != username {
                                return Err("Username doesn't match this ID".to_string());
                            }
                            // Verify password
                            match firebase.verify_password(&user_id, &password).await {
                                Ok(true) => {
                                    // Update status to Online and set current IP
                                    let local_ip = get_local_ip();
                                    let _ = firebase.update_user_status(&user_id, &UserStatus::Online).await;
                                    let _ = firebase.update_last_seen(&user_id, chrono::Utc::now().timestamp()).await;
                                    let _ = firebase.update_user_ip(&user_id, &local_ip).await;
                                    Ok((username, user_id))
                                }
                                Ok(false) => Err("Incorrect password".to_string()),
                                Err(e) => Err(format!("Error verifying password: {}", e)),
                            }
                        }
                        Ok(None) => Err("User ID not found. Click 'Register' to create an account.".to_string()),
                        Err(e) => Err(format!("Network error: {}", e)),
                    }
                }
            })
        });

        self.login_in_progress = Some(promise);
        self.login_error = None;
    }

    fn render_login_screen(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);

            ui.heading(RichText::new("🖼️  Distributed Image Cloud").size(28.0));
            ui.add_space(10.0);
            
            let mode_text = if self.is_registering { "Create a new account" } else { "Sign in to your account" };
            ui.label(RichText::new(mode_text).size(14.0));

            ui.add_space(30.0);

            // Username field (different label/hint based on mode)
            ui.horizontal(|ui| {
                ui.add_space(200.0);
                let (label, hint) = if self.is_registering {
                    ("Username:", "Choose a username")
                } else {
                    ("Username#ID:", "e.g., potato#1")
                };
                ui.label(RichText::new(label).size(16.0));
                ui.add(
                    egui::TextEdit::singleline(&mut self.username_input)
                        .desired_width(250.0)
                        .hint_text(hint)
                );
            });

            ui.add_space(10.0);

            // Password field
            ui.horizontal(|ui| {
                ui.add_space(200.0);
                ui.label(RichText::new("Password:").size(16.0));
                ui.add(
                    egui::TextEdit::singleline(&mut self.password_input)
                        .desired_width(250.0)
                        .password(true)
                        .hint_text("Enter your password")
                );
            });

            ui.add_space(20.0);

            // Show error message if any
            if let Some(error) = &self.login_error {
                ui.label(RichText::new(error).color(Color32::from_rgb(255, 50, 50)).size(14.0));
                ui.add_space(10.0);
            }

            // Login/Register button or progress
            let mut should_clear_progress = false;
            let was_registering = self.is_registering;
            if let Some(promise) = &self.login_in_progress {
                match promise.ready() {
                    None => {
                        ui.horizontal(|ui| {
                            ui.add_space(310.0);
                            ui.spinner();
                            let action = if self.is_registering { "Registering..." } else { "Logging in..." };
                            ui.label(action);
                        });
                    }
                    Some(result) => {
                        match result {
                            Ok((username, user_id)) => {
                                // Log in immediately (for both registration and login)
                                self.is_logged_in = true;
                                self.username = username.clone();
                                self.user_id = user_id.clone();
                                self.password_input.clear();
                                
                                // If just registered, set flag to show ID popup
                                if was_registering {
                                    self.just_registered_id = Some(user_id.clone());
                                }
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
                    ui.add_space(270.0);
                    
                    let button_text = if self.is_registering { "📝 Register" } else { "🔑 Login" };
                    if ui.button(RichText::new(button_text).size(16.0)).clicked() {
                        self.attempt_login();
                    }
                });
            }

            if should_clear_progress {
                self.login_in_progress = None;
            }

            ui.add_space(20.0);

            // Toggle between login and register
            ui.horizontal(|ui| {
                ui.add_space(220.0);
                if self.is_registering {
                    ui.label("Already have an account?");
                    if ui.link("Login").clicked() {
                        self.is_registering = false;
                        self.login_error = None;
                    }
                } else {
                    ui.label("Don't have an account?");
                    if ui.link("Register").clicked() {
                        self.is_registering = true;
                        self.login_error = None;
                    }
                }
            });
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
                        let size_kb = metadata.len() as f64 / 1024.0;
                        let size_mb = size_kb / 1024.0;

                        let (size_text, color) = if size_mb >= 1.0 {
                            (format!("Size: {:.2} MB", size_mb), Color32::from_rgb(255, 165, 0))
                        } else {
                            (format!("Size: {:.2} KB", size_kb), Color32::from_rgb(0, 200, 0))
                        };

                        ui.label(RichText::new(size_text).color(color));
                        if size_kb > 100.0 {
                            let chunks = ((size_kb * 1024.0) / 45000.0).ceil() as usize;
                            ui.label(RichText::new(format!("ℹ️ Will be transmitted as {} chunks", chunks)).color(Color32::from_rgb(100, 150, 255)).size(11.0));
                        }
                    }
                } else {
                    ui.label(RichText::new("No image selected").color(Color32::GRAY));
                }
            });

            ui.label(RichText::new("ℹ️ Images are automatically chunked for UDP transmission (no size limit)").color(Color32::from_rgb(100, 150, 255)).size(11.0));

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

        let quota: u8 = self.viewing_quota;
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

            // UDP chunking system handles images of any size
            // Images are automatically fragmented into 45KB chunks and reassembled
            eprintln!("[DEBUG] Image size: {} bytes ({:.2} KB)", image_data.len(), image_data.len() as f32 / 1024.0);

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

        // Send heartbeat every 30 seconds while logged in
        if self.is_logged_in {
            if let Some(last) = self.last_heartbeat {
                if last.elapsed().as_secs() >= 30 {
                    self.send_heartbeat();
                    self.last_heartbeat = Some(std::time::Instant::now());
                }
            }
        }

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
                    // Logout button
                    if ui.button("🚪 Logout").clicked() {
                        self.logout();
                    }

                    ui.separator();

                    if ui.button("❓ Help").clicked() {
                        self.show_help = !self.show_help;
                    }

                    ui.separator();

                    ui.label(RichText::new(format!("👤 {} (ID: {})", self.username, self.user_id))
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

        // Show welcome popup with ID after registration
        if self.just_registered_id.is_some() {
            egui::Window::new("🎉 Welcome!")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.label(RichText::new("Registration successful!").size(18.0).color(Color32::from_rgb(0, 200, 0)));
                        ui.add_space(15.0);
                        ui.label(RichText::new(format!("Your ID is: {}", self.just_registered_id.as_ref().unwrap())).size(24.0).strong());
                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("Login with: {}#{}", self.username, self.user_id)).size(14.0));
                        ui.add_space(5.0);
                        ui.label(RichText::new("Remember this for future logins!").size(12.0).color(Color32::GRAY));
                        ui.add_space(15.0);
                        if ui.button(RichText::new("  Got it!  ").size(16.0)).clicked() {
                            self.just_registered_id = None;
                        }
                        ui.add_space(10.0);
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, Tab::Upload, "📤 Upload");
                ui.selectable_value(&mut self.selected_tab, Tab::ReceivedImages, "📬 Received");
                ui.selectable_value(&mut self.selected_tab, Tab::MyGallery, "🖼️ My Gallery");
                ui.selectable_value(&mut self.selected_tab, Tab::BrowseUsers, "🔍 Browse Users");
                ui.selectable_value(&mut self.selected_tab, Tab::History, "📜 History");
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "⚙️ Settings");
            });

            ui.separator();
            ui.add_space(10.0);

            match self.selected_tab {
                Tab::Upload => self.render_upload_tab(ui, ctx),
                Tab::ReceivedImages => self.render_received_images_tab(ui, ctx),
                Tab::MyGallery => self.render_my_gallery_tab(ui, ctx),
                Tab::BrowseUsers => self.render_browse_users_tab(ui, ctx),
                Tab::History => self.render_history_tab(ui),
                Tab::Settings => self.render_settings_tab(ui),
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Unregister session when app closes
        if self.is_logged_in && !self.username.is_empty() {
            let user_id = self.user_id.clone();
            let runtime = self.runtime.as_ref().unwrap().clone();

            // Update status to Offline in Firebase
            runtime.block_on(async move {
                let firebase = FireBaseClient::new();
                let _ = firebase.update_user_status(&user_id, &UserStatus::Offline).await;
            });
        }
    }
}

// Additional ClientApp methods for gallery and user browsing
impl ClientApp {
    fn render_my_gallery_tab(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.heading("🖼️ My Gallery");
        ui.add_space(5.0);
        ui.label(RichText::new("Share up to 5 images in your public gallery (images are pixelated for privacy)").color(Color32::GRAY).size(12.0));
        ui.add_space(10.0);

        // Load gallery if not loaded yet
        if !self.gallery_loaded && self.gallery_loading.is_none() {
            self.load_my_gallery();
        }

        // Check loading status
        let mut should_clear_loading = false;
        if let Some(promise) = &self.gallery_loading {
            match promise.ready() {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading your gallery...");
                    });
                }
                Some(result) => {
                    match result {
                        Ok(gallery) => {
                            self.my_gallery = gallery.clone();
                            self.my_gallery_textures = vec![None; gallery.len()];
                            self.gallery_loaded = true;
                        }
                        Err(e) => {
                            self.gallery_error = Some(e.clone());
                            self.gallery_loaded = true;  // Mark as loaded even on error to stop retrying
                        }
                    }
                    should_clear_loading = true;
                }
            }
        }
        if should_clear_loading {
            self.gallery_loading = None;
        }

        // Show error if any
        if let Some(error) = &self.gallery_error {
            ui.label(RichText::new(error).color(Color32::RED));
            ui.add_space(5.0);
        }

        // Gallery grid
        ui.group(|ui| {
            ui.label(RichText::new(format!("Your Images ({}/5)", self.my_gallery.len())).size(14.0).strong());
            ui.add_space(10.0);

            if self.my_gallery.is_empty() {
                ui.label(RichText::new("No images in your gallery yet").color(Color32::GRAY));
            } else {
                ui.horizontal_wrapped(|ui| {
                    let mut to_remove: Option<usize> = None;
                    for (i, img_url) in self.my_gallery.iter().enumerate() {
                        ui.vertical(|ui| {
                            // Try to load texture if not already loaded
                            if self.my_gallery_textures.get(i).map(|t| t.is_none()).unwrap_or(true) {
                                if let Some(color_image) = decode_base64_image(img_url) {
                                    let texture = ctx.load_texture(
                                        format!("my_gallery_{}", i),
                                        color_image,
                                        egui::TextureOptions::LINEAR,
                                    );
                                    if i < self.my_gallery_textures.len() {
                                        self.my_gallery_textures[i] = Some(texture);
                                    }
                                }
                            }

                            // Display image or placeholder
                            let size = egui::vec2(100.0, 100.0);
                            if let Some(Some(texture)) = self.my_gallery_textures.get(i) {
                                ui.image((texture.id(), size));
                            } else {
                                let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
                                ui.painter().rect_filled(rect, 5.0, Color32::from_rgb(60, 60, 60));
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("#{}", i + 1),
                                    egui::FontId::proportional(14.0),
                                    Color32::WHITE,
                                );
                            }

                            if ui.button("🗑️ Remove").clicked() {
                                to_remove = Some(i);
                            }
                        });
                        ui.add_space(10.0);
                    }
                    if let Some(idx) = to_remove {
                        self.remove_from_gallery(idx);
                    }
                });
            }
        });

        ui.add_space(15.0);

        // Add image button
        let can_add = self.my_gallery.len() < 5 && self.gallery_upload_in_progress.is_none();
        ui.add_enabled_ui(can_add, |ui| {
            if ui.button(RichText::new("➕ Add Image to Gallery").size(14.0)).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                    .pick_file()
                {
                    self.add_to_gallery(path, ctx);
                }
            }
        });

        if !can_add && self.my_gallery.len() >= 5 {
            ui.label(RichText::new("⚠️ Gallery is full (max 5 images)").color(Color32::from_rgb(255, 165, 0)));
        }

        // Upload progress
        let mut should_clear_upload = false;
        if let Some(promise) = &self.gallery_upload_in_progress {
            match promise.ready() {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Uploading image...");
                    });
                }
                Some(result) => {
                    match result {
                        Ok(()) => {
                            self.gallery_error = None;
                            // Reload gallery
                            self.load_my_gallery();
                        }
                        Err(e) => {
                            self.gallery_error = Some(e.clone());
                        }
                    }
                    should_clear_upload = true;
                }
            }
        }
        if should_clear_upload {
            self.gallery_upload_in_progress = None;
        }

        ui.add_space(10.0);
        if ui.button("🔄 Refresh Gallery").clicked() {
            self.load_my_gallery();
        }
    }

    fn render_browse_users_tab(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.heading("🔍 Browse Users");
        ui.add_space(10.0);

        // Search bar
        ui.group(|ui| {
            ui.label(RichText::new("Search for a user").size(14.0).strong());
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.user_search_input)
                        .desired_width(300.0)
                        .hint_text("Enter username or username#id")
                );

                let can_search = !self.user_search_input.trim().is_empty() 
                    && self.user_search_in_progress.is_none();

                if ui.add_enabled(can_search, egui::Button::new("🔍 Search")).clicked()
                    || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_search)
                {
                    self.search_user();
                }
            });
        });

        // Search progress/error
        let mut should_clear_search = false;
        if let Some(promise) = &self.user_search_in_progress {
            match promise.ready() {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Searching...");
                    });
                }
                Some(result) => {
                    match result {
                        Ok(Some((user_id, user_info))) => {
                            self.searched_user = Some(user_info.clone());
                            self.searched_user_id = Some(user_id.clone());
                            self.viewed_gallery = user_info.gallery.clone();
                            self.viewed_gallery_textures = vec![None; user_info.gallery.len()];
                            self.user_search_error = None;
                        }
                        Ok(None) => {
                            self.searched_user = None;
                            self.searched_user_id = None;
                            self.viewed_gallery.clear();
                            self.user_search_error = Some("User not found".to_string());
                        }
                        Err(e) => {
                            self.user_search_error = Some(e.clone());
                        }
                    }
                    should_clear_search = true;
                }
            }
        }
        if should_clear_search {
            self.user_search_in_progress = None;
        }

        // Show error if any
        if let Some(error) = &self.user_search_error {
            ui.add_space(10.0);
            ui.label(RichText::new(error).color(Color32::RED));
        }

        ui.add_space(15.0);

        // Show searched user info
        if let Some(user) = &self.searched_user {
            let user_id = self.searched_user_id.clone().unwrap_or_default();
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("👤").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("{}#{}", &user.username, &user_id)).size(18.0).strong());
                        
                        // Determine real status based on last_seen (offline if inactive > 60 seconds)
                        let now = chrono::Utc::now().timestamp();
                        let seconds_since_seen = now - user.last_seen;
                        let is_really_online = seconds_since_seen < 60;
                        
                        let (status_color, status_text) = if !is_really_online {
                            (Color32::GRAY, "⚫ Offline")
                        } else {
                            match user.status {
                                UserStatus::Online => (Color32::from_rgb(0, 200, 0), "🟢 Online"),
                                UserStatus::Offline => (Color32::GRAY, "⚫ Offline"),
                                UserStatus::Idle => (Color32::from_rgb(255, 165, 0), "🟡 Idle"),
                            }
                        };
                        ui.label(RichText::new(status_text).color(status_color));
                        
                        // Show last seen time if offline
                        if !is_really_online {
                            let mins_ago = seconds_since_seen / 60;
                            let last_seen_text = if mins_ago < 60 {
                                format!("Last seen {} min ago", mins_ago)
                            } else if mins_ago < 1440 {
                                format!("Last seen {} hours ago", mins_ago / 60)
                            } else {
                                format!("Last seen {} days ago", mins_ago / 1440)
                            };
                            ui.label(RichText::new(last_seen_text).color(Color32::GRAY).size(11.0));
                        }
                    });
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // User's gallery
                ui.label(RichText::new(format!("Gallery ({} images)", self.viewed_gallery.len())).size(14.0).strong());
                ui.add_space(5.0);

                if self.viewed_gallery.is_empty() {
                    ui.label(RichText::new("This user has no public gallery images").color(Color32::GRAY));
                } else {
                    ui.horizontal_wrapped(|ui| {
                        for (i, img_url) in self.viewed_gallery.iter().enumerate() {
                            // Try to load texture if not already loaded
                            if self.viewed_gallery_textures.get(i).map(|t| t.is_none()).unwrap_or(true) {
                                if let Some(color_image) = decode_base64_image(img_url) {
                                    let texture = ctx.load_texture(
                                        format!("viewed_gallery_{}", i),
                                        color_image,
                                        egui::TextureOptions::LINEAR,
                                    );
                                    if i < self.viewed_gallery_textures.len() {
                                        self.viewed_gallery_textures[i] = Some(texture);
                                    }
                                }
                            }

                            // Display image or placeholder
                            let size = egui::vec2(120.0, 120.0);
                            if let Some(Some(texture)) = self.viewed_gallery_textures.get(i) {
                                ui.image((texture.id(), size));
                            } else {
                                let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::click());
                                ui.painter().rect_filled(rect, 5.0, Color32::from_rgb(80, 80, 80));
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("Image #{}", i + 1),
                                    egui::FontId::proportional(12.0),
                                    Color32::WHITE,
                                );
                            }
                            ui.add_space(5.0);
                        }
                    });
                }
            });
        }
    }

    fn load_my_gallery(&mut self) {
        let user_id = self.user_id.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("load_gallery", move || {
            let firebase = FireBaseClient::new();
            runtime.block_on(async move {
                firebase.get_user_gallery(&user_id).await
                    .map_err(|e| format!("Failed to load gallery: {}", e))
            })
        });

        self.gallery_loading = Some(promise);
    }

    fn add_to_gallery(&mut self, path: std::path::PathBuf, _ctx: &egui::Context) {
        let user_id = self.user_id.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();
        let mut current_gallery = self.my_gallery.clone();

        let promise = Promise::spawn_thread("upload_gallery", move || {
            // Read and pixelate the image
            let img = image::open(&path)
                .map_err(|e| format!("Failed to open image: {}", e))?;

            // Pixelate by reducing resolution to 64x64 then scaling back
            let pixelated = img.resize_exact(64, 64, image::imageops::FilterType::Nearest);
            
            // Convert to base64 for storage
            let mut buf = Vec::new();
            pixelated.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .map_err(|e| format!("Failed to encode image: {}", e))?;
            
            let base64_img = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &buf);
            let data_url = format!("data:image/png;base64,{}", base64_img);

            // Add to gallery
            current_gallery.push(data_url);

            let firebase = FireBaseClient::new();
            runtime.block_on(async move {
                firebase.update_gallery(&user_id, &current_gallery).await
                    .map_err(|e| format!("Failed to update gallery: {}", e))
            })
        });

        self.gallery_upload_in_progress = Some(promise);
    }

    fn remove_from_gallery(&mut self, index: usize) {
        let user_id = self.user_id.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();
        let mut current_gallery = self.my_gallery.clone();

        if index < current_gallery.len() {
            current_gallery.remove(index);
        }

        let promise = Promise::spawn_thread("remove_gallery", move || {
            let firebase = FireBaseClient::new();
            runtime.block_on(async move {
                firebase.update_gallery(&user_id, &current_gallery).await
                    .map_err(|e| format!("Failed to update gallery: {}", e))
            })
        });

        self.gallery_upload_in_progress = Some(promise);
        self.my_gallery.remove(index);
        if index < self.my_gallery_textures.len() {
            self.my_gallery_textures.remove(index);
        }
    }

    fn search_user(&mut self) {
        let input = self.user_search_input.trim().to_string();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("search_user", move || {
            let firebase = FireBaseClient::new();
            runtime.block_on(async move {
                // Check if input contains # (username#id format)
                if input.contains('#') {
                    let parts: Vec<&str> = input.split('#').collect();
                    if parts.len() == 2 {
                        let username = parts[0].to_string();
                        let user_id = parts[1].to_string();
                        
                        // Search by ID directly
                        match firebase.get_user(&user_id).await {
                            Ok(Some(user_info)) => {
                                // Verify username matches
                                if user_info.username == username {
                                    Ok(Some((user_id, user_info)))
                                } else {
                                    Ok(None) // Username doesn't match this ID
                                }
                            }
                            Ok(None) => Ok(None),
                            Err(e) => Err(format!("Search failed: {}", e)),
                        }
                    } else {
                        // Invalid format, try as username
                        firebase.find_user_by_username(&input).await
                            .map_err(|e| format!("Search failed: {}", e))
                    }
                } else {
                    // Search by username only
                    firebase.find_user_by_username(&input).await
                        .map_err(|e| format!("Search failed: {}", e))
                }
            })
        });

        self.user_search_in_progress = Some(promise);
        self.searched_user = None;
        self.searched_user_id = None;
        self.viewed_gallery.clear();
    }
}
