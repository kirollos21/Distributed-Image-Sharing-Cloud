// New clean GUI client for Distributed Image Cloud
// Features: Login, Register, Send Images, Inbox, My Gallery, Browse Users, Settings

use crate::client::Client;
use crate::firebase::{FireBaseClient, UserInfo, UserStatus, ReceivedImageMeta};
use crate::messages::Message;
use eframe::egui;
use egui::{Color32, RichText, Vec2, Rounding};
use poll_promise::Promise;
use std::sync::Arc;

// ============================================================================
// App State
// ============================================================================

#[derive(Default, PartialEq, Clone)]
pub enum Page {
    #[default]
    Login,
    SendImage,
    Inbox,       // Images sent TO you (private, with view quota)
    MyGallery,   // Your public gallery (up to 5 pixelated images)
    BrowseUsers, // Search users and see their galleries
    Settings,
}

#[derive(Default, Clone)]
pub struct UserSession {
    pub username: String,
    pub user_id: String,
    pub is_logged_in: bool,
}

// ============================================================================
// Main App
// ============================================================================

pub struct ClientAppV2 {
    // Runtime
    runtime: Option<Arc<tokio::runtime::Runtime>>,
    cloud_addresses: Vec<String>,
    firebase_client: FireBaseClient,
    
    // User session
    session: UserSession,
    
    // Navigation
    current_page: Page,
    
    // Login/Register state
    username_input: String,
    password_input: String,
    is_registering: bool,
    auth_in_progress: Option<Promise<Result<(String, String, String), String>>>, // (username, id, message)
    auth_error: Option<String>,
    auth_success: Option<String>,  // For showing registration ID
    
    // Send Image state
    selected_image_path: Option<String>,
    recipient_input: String,
    recipients: Vec<String>,
    view_quota: u8,
    send_in_progress: Option<Promise<Result<String, String>>>,
    send_result: Option<Result<String, String>>,
    recipient_check: Option<Promise<Result<bool, String>>>,
    recipient_error: Option<String>,
    
    // Inbox state (received images from local cache)
    received_images: Vec<ReceivedImage>,
    received_images_loading: bool,
    selected_received_image: Option<usize>,
    viewing_image: Option<Vec<u8>>,  // Decrypted image being viewed
    viewing_image_texture: Option<egui::TextureHandle>,  // Texture for display
    viewing_in_progress: Option<Promise<Result<(Vec<u8>, u8), String>>>,  // (decrypted_data, remaining_views)
    view_error: Option<String>,
    
    // My Gallery state (public pixelated images - max 5)
    my_gallery: Vec<String>,           // Base64 data URLs of pixelated images
    my_gallery_loaded: bool,
    my_gallery_loading: Option<Promise<Result<Vec<String>, String>>>,
    my_gallery_textures: Vec<Option<egui::TextureHandle>>,
    gallery_upload_path: Option<String>,
    gallery_upload_in_progress: Option<Promise<Result<(), String>>>,
    gallery_error: Option<String>,
    
    // Browse Users state
    search_query: String,
    search_results: Vec<UserSearchResult>,
    search_in_progress: Option<Promise<Result<Vec<UserSearchResult>, String>>>,
    search_gallery_textures: std::collections::HashMap<String, Vec<egui::TextureHandle>>,  // user_id -> textures
    
    // Settings state
    new_username_input: String,
    username_change_in_progress: Option<Promise<Result<(), String>>>,
    settings_message: Option<(String, bool)>,  // (message, is_error)
    
    // Heartbeat & polling
    last_heartbeat: Option<std::time::Instant>,
    last_poll: Option<std::time::Instant>,
    poll_in_progress: Option<Promise<Result<u32, String>>>,  // Number of new images received
}

/// Represents a received encrypted image stored on disk
#[derive(Clone)]
pub struct ReceivedImage {
    pub filename: String,
    pub from_user: String,
    pub remaining_views: u8,
    pub max_views: u8,
    pub received_at: i64,
    pub file_path: String,
    pub image_id: String,
}

#[derive(Clone)]
pub struct UserSearchResult {
    pub username: String,
    pub user_id: String,
    pub status: String,
    pub last_seen: i64,
    pub gallery: Vec<String>,  // Base64 data URLs of pixelated gallery images
}

// ============================================================================
// Colors & Styling
// ============================================================================

struct AppColors;
impl AppColors {
    const PRIMARY: Color32 = Color32::from_rgb(79, 70, 229);      // Indigo
    const PRIMARY_HOVER: Color32 = Color32::from_rgb(99, 90, 249);
    const SECONDARY: Color32 = Color32::from_rgb(59, 130, 246);   // Blue
    const SUCCESS: Color32 = Color32::from_rgb(34, 197, 94);      // Green
    const ERROR: Color32 = Color32::from_rgb(239, 68, 68);        // Red
    const WARNING: Color32 = Color32::from_rgb(234, 179, 8);      // Yellow
    const BG_DARK: Color32 = Color32::from_rgb(17, 24, 39);       // Dark background
    const BG_CARD: Color32 = Color32::from_rgb(31, 41, 55);       // Card background
    const BG_INPUT: Color32 = Color32::from_rgb(55, 65, 81);      // Input background
    const TEXT_PRIMARY: Color32 = Color32::from_rgb(243, 244, 246);
    const TEXT_SECONDARY: Color32 = Color32::from_rgb(156, 163, 175);
    const BORDER: Color32 = Color32::from_rgb(75, 85, 99);
}

// ============================================================================
// Implementation
// ============================================================================

impl ClientAppV2 {
    /// Get the path to the received images cache directory (shared)
    fn get_cache_dir() -> std::path::PathBuf {
        let cache_dir = std::path::PathBuf::from("./received_images");
        if !cache_dir.exists() {
            let _ = std::fs::create_dir_all(&cache_dir);
        }
        cache_dir
    }
    
    /// Get the path to a user-specific cache directory
    fn get_user_cache_dir(user_id: &str) -> std::path::PathBuf {
        let cache_dir = std::path::PathBuf::from(format!("./received_images/{}", user_id));
        if !cache_dir.exists() {
            let _ = std::fs::create_dir_all(&cache_dir);
        }
        cache_dir
    }

    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = Arc::new(
            tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"),
        );

        // Create cache directory
        let _ = Self::get_cache_dir();

        // Fetch node addresses from Firebase
        eprintln!("Fetching node addresses from Firebase...");
        let firebase = FireBaseClient::new();
        let cloud_addresses = runtime.block_on(async {
            match firebase.get_online_node_addresses().await {
                Ok(addrs) if !addrs.is_empty() => {
                    eprintln!("✓ Found {} online nodes: {:?}", addrs.len(), addrs);
                    addrs
                }
                _ => {
                    eprintln!("⚠ Using localhost defaults");
                    vec!["127.0.0.1:8001".to_string()]
                }
            }
        });

        Self {
            runtime: Some(runtime),
            cloud_addresses,
            firebase_client: FireBaseClient::new(),
            session: UserSession::default(),
            current_page: Page::Login,
            username_input: String::new(),
            password_input: String::new(),
            is_registering: false,
            auth_in_progress: None,
            auth_error: None,
            auth_success: None,
            selected_image_path: None,
            recipient_input: String::new(),
            recipients: Vec::new(),
            view_quota: 5,
            send_in_progress: None,
            send_result: None,
            recipient_check: None,
            recipient_error: None,
            received_images: Vec::new(),
            received_images_loading: false,
            selected_received_image: None,
            viewing_image: None,
            viewing_image_texture: None,
            viewing_in_progress: None,
            view_error: None,
            my_gallery: Vec::new(),
            my_gallery_loaded: false,
            my_gallery_loading: None,
            my_gallery_textures: Vec::new(),
            gallery_upload_path: None,
            gallery_upload_in_progress: None,
            gallery_error: None,
            search_query: String::new(),
            search_results: Vec::new(),
            search_in_progress: None,
            search_gallery_textures: std::collections::HashMap::new(),
            new_username_input: String::new(),
            username_change_in_progress: None,
            settings_message: None,
            last_heartbeat: None,
            last_poll: None,
            poll_in_progress: None,
        }
    }

    fn send_heartbeat(&self) {
        if !self.session.is_logged_in || self.session.user_id.is_empty() {
            return;
        }
        
        let user_id = self.session.user_id.clone();
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        std::thread::spawn(move || {
            let client = Client::new(0, cloud_addresses);
            let _ = runtime.block_on(async move {
                client.client_heartbeat(user_id).await
            });
        });
    }

    fn logout(&mut self) {
        let user_id = self.session.user_id.clone();
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        // Send logout to node
        std::thread::spawn(move || {
            let client = Client::new(0, cloud_addresses);
            let _ = runtime.block_on(async move {
                client.client_logout(user_id).await
            });
        });

        // Reset state
        self.session = UserSession::default();
        self.current_page = Page::Login;
        self.auth_error = None;
        self.auth_success = None;
        self.recipients.clear();
        self.received_images.clear();
        self.search_results.clear();
    }

    /// Poll node for pending images and save them to local cache
    fn poll_for_pending_images(&mut self) {
        if self.poll_in_progress.is_some() {
            return;
        }

        let username = format!("{}#{}", self.session.username, self.session.user_id);
        let user_id = self.session.user_id.clone();
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("poll_images", move || {
            let cache_dir = Self::get_user_cache_dir(&user_id);
            let user_id_clone = user_id.clone();
            
            runtime.block_on(async move {
                let client = Client::new(0, cloud_addresses.clone());
                let firebase = FireBaseClient::new();
                
                // Query for received images from node
                match client.query_received_images(username.clone()).await {
                    Ok(images) => {
                        let mut saved_count = 0u32;
                        
                        for img_info in images {
                            // Check if we already have this image cached
                            let filename = format!("{}_{}.enc", img_info.from_username.replace('#', "_"), img_info.image_id);
                            let file_path = cache_dir.join(&filename);
                            
                            if !file_path.exists() {
                                // Fetch the encrypted image data (without decrypting)
                                match client.download_image_encrypted(username.clone(), img_info.image_id.clone()).await {
                                    Ok(encrypted_data) => {
                                        // Save metadata alongside the image
                                        let meta = ImageCacheMeta {
                                            from_user: img_info.from_username.clone(),
                                            remaining_views: img_info.remaining_views,
                                            max_views: img_info.remaining_views,
                                            received_at: img_info.timestamp,
                                            image_id: img_info.image_id.clone(),
                                        };
                                        
                                        // Save encrypted image locally
                                        if std::fs::write(&file_path, &encrypted_data).is_ok() {
                                            // Save metadata locally
                                            let meta_path = file_path.with_extension("meta.json");
                                            let _ = std::fs::write(&meta_path, serde_json::to_string(&meta).unwrap_or_default());
                                            saved_count += 1;
                                            
                                            // Also upload to Firebase for cross-device sync
                                            let firebase_meta = ReceivedImageMeta {
                                                image_id: img_info.image_id.clone(),
                                                from_user: img_info.from_username.clone(),
                                                remaining_views: img_info.remaining_views,
                                                max_views: img_info.remaining_views,
                                                received_at: img_info.timestamp,
                                                encrypted_data_base64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &encrypted_data),
                                            };
                                            let _ = firebase.add_received_image(&user_id_clone, &firebase_meta).await;
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                        
                        Ok(saved_count)
                    }
                    Err(e) => Err(e),
                }
            })
        });

        self.poll_in_progress = Some(promise);
    }

    /// Load received images from local cache directory
    /// Load received images from local cache directory
    fn load_cached_images(&mut self) {
        self.received_images.clear();
        let cache_dir = Self::get_user_cache_dir(&self.session.user_id);

        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "enc") {
                    let meta_path = path.with_extension("meta.json");
                    
                    if let Ok(meta_content) = std::fs::read_to_string(&meta_path) {
                        if let Ok(meta) = serde_json::from_str::<ImageCacheMeta>(&meta_content) {
                            self.received_images.push(ReceivedImage {
                                filename: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                                from_user: meta.from_user,
                                remaining_views: meta.remaining_views,
                                max_views: meta.max_views,
                                received_at: meta.received_at,
                                file_path: path.to_string_lossy().to_string(),
                                image_id: meta.image_id,
                            });
                        }
                    }
                }
            }
        }
        
        // Sort by received time (newest first)
        self.received_images.sort_by(|a, b| b.received_at.cmp(&a.received_at));
    }

    /// Sync received images from Firebase (for cross-device access)
    /// Downloads any images stored in Firebase that aren't in the local cache
    fn sync_from_firebase(&mut self) {
        let user_id = self.session.user_id.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();
        
        // Run sync in background thread (fire-and-forget)
        std::thread::spawn(move || {
            let cache_dir = Self::get_user_cache_dir(&user_id);
            
            runtime.block_on(async move {
                let firebase = FireBaseClient::new();
                
                // Get all images from Firebase
                if let Ok(firebase_images) = firebase.get_received_images(&user_id).await {
                    for img in firebase_images {
                        // Check if we already have this image locally
                        let filename = format!("{}_{}.enc", img.from_user.replace('#', "_"), img.image_id);
                        let file_path = cache_dir.join(&filename);
                        
                        if !file_path.exists() {
                            // Decode and save to local cache
                            if let Ok(encrypted_data) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &img.encrypted_data_base64) {
                                // Save encrypted image
                                if std::fs::write(&file_path, &encrypted_data).is_ok() {
                                    // Save metadata
                                    let meta = ImageCacheMeta {
                                        from_user: img.from_user.clone(),
                                        remaining_views: img.remaining_views,
                                        max_views: img.max_views,
                                        received_at: img.received_at,
                                        image_id: img.image_id.clone(),
                                    };
                                    let meta_path = file_path.with_extension("meta.json");
                                    let _ = std::fs::write(&meta_path, serde_json::to_string(&meta).unwrap_or_default());
                                }
                            }
                        }
                    }
                }
            });
        });
    }

    fn process_poll_result(&mut self) {
        let result = if let Some(promise) = &self.poll_in_progress {
            promise.ready().cloned()
        } else {
            None
        };

        if let Some(res) = result {
            if let Ok(count) = res {
                if count > 0 {
                    // Reload cached images to show new ones
                    self.load_cached_images();
                }
            }
            self.poll_in_progress = None;
        }
    }

    /// View a cached image - decrypt it and update view count
    fn view_cached_image(&mut self, index: usize) {
        if self.viewing_in_progress.is_some() {
            return; // Already viewing
        }
        
        let image = match self.received_images.get(index) {
            Some(img) => img.clone(),
            None => return,
        };
        
        if image.remaining_views == 0 {
            self.view_error = Some("No views remaining".to_string());
            return;
        }
        
        let file_path = image.file_path.clone();
        let meta_path = std::path::PathBuf::from(&file_path).with_extension("meta.json");
        let user_id = self.session.user_id.clone();
        let image_id = image.image_id.clone();
        
        let promise = Promise::spawn_thread("view_image", move || {
            // Read encrypted image from disk
            let encrypted_data = std::fs::read(&file_path)
                .map_err(|e| format!("Failed to read image: {}", e))?;
            
            // Update view count and get the updated encrypted image
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Failed to create runtime: {}", e))?;
            
            let (updated_encrypted, metadata, success) = rt.block_on(async {
                crate::encryption::update_view_count(&encrypted_data).await
            })?;
            
            if !success {
                return Err("View quota exhausted".to_string());
            }
            
            // Save the updated encrypted image back to disk
            std::fs::write(&file_path, &updated_encrypted)
                .map_err(|e| format!("Failed to update image file: {}", e))?;
            
            // Update metadata file with new view count
            let remaining = metadata.quota - metadata.viewed;
            if let Ok(meta_content) = std::fs::read_to_string(&meta_path) {
                if let Ok(mut meta) = serde_json::from_str::<ImageCacheMeta>(&meta_content) {
                    meta.remaining_views = remaining;
                    let _ = std::fs::write(&meta_path, serde_json::to_string(&meta).unwrap_or_default());
                }
            }
            
            // Update Firebase with new view count (for cross-device sync)
            rt.block_on(async {
                let firebase = FireBaseClient::new();
                if remaining > 0 {
                    let _ = firebase.update_received_image_views(&user_id, &image_id, remaining).await;
                } else {
                    // No views left, delete from Firebase
                    let _ = firebase.delete_received_image(&user_id, &image_id).await;
                }
            });
            
            // Decrypt to get the viewable image
            let (decrypted_image, _) = rt.block_on(async {
                crate::encryption::decrypt_image(updated_encrypted).await
            })?;
            
            Ok((decrypted_image, remaining))
        });
        
        self.viewing_in_progress = Some(promise);
        self.selected_received_image = Some(index);
    }

    /// Process viewing result
    fn process_view_result(&mut self, ctx: &egui::Context) {
        let result = if let Some(promise) = &self.viewing_in_progress {
            promise.ready().cloned()
        } else {
            None
        };

        if let Some(res) = result {
            match res {
                Ok((image_data, remaining_views)) => {
                    // Load the decrypted image as a texture
                    if let Ok(img) = image::load_from_memory(&image_data) {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                        let texture = ctx.load_texture("viewed_image", color_image, egui::TextureOptions::default());
                        self.viewing_image_texture = Some(texture);
                        self.viewing_image = Some(image_data);
                        self.view_error = None;
                        
                        // Update remaining views in our list
                        if let Some(idx) = self.selected_received_image {
                            if let Some(img) = self.received_images.get_mut(idx) {
                                img.remaining_views = remaining_views;
                            }
                        }
                    } else {
                        self.view_error = Some("Failed to decode image".to_string());
                    }
                }
                Err(e) => {
                    self.view_error = Some(e);
                }
            }
            self.viewing_in_progress = None;
        }
    }

    /// Close the image viewer
    fn close_image_viewer(&mut self) {
        self.viewing_image = None;
        self.viewing_image_texture = None;
        self.selected_received_image = None;
        self.view_error = None;
    }
}

/// Metadata stored alongside cached encrypted images
#[derive(serde::Serialize, serde::Deserialize)]
struct ImageCacheMeta {
    from_user: String,
    remaining_views: u8,
    max_views: u8,
    received_at: i64,
    image_id: String,
}

// ============================================================================
// UI Rendering
// ============================================================================

impl eframe::App for ClientAppV2 {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set dark theme
        ctx.set_visuals(egui::Visuals::dark());
        
        // Heartbeat every 70 seconds (for "last seen" info) + poll for images every 30 seconds
        if self.session.is_logged_in {
            let now = std::time::Instant::now();
            if self.last_heartbeat.map_or(true, |t| now.duration_since(t).as_secs() >= 70) {
                self.send_heartbeat();
                self.last_heartbeat = Some(now);
            }
            
            // Poll for pending images every 30 seconds
            if self.last_poll.map_or(true, |t| now.duration_since(t).as_secs() >= 30) {
                self.poll_for_pending_images();
                self.last_poll = Some(now);
            }
            
            // Process poll results
            self.process_poll_result();
            
            // Process view image results
            self.process_view_result(ctx);
        }

        // Main panel with dark background
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(AppColors::BG_DARK))
            .show(ctx, |ui| {
                if !self.session.is_logged_in {
                    self.render_auth_page(ui, ctx);
                } else {
                    self.render_main_app(ui, ctx);
                }
            });

        // Image viewer popup window
        if self.viewing_image.is_some() || self.viewing_in_progress.is_some() {
            self.render_image_viewer(ctx);
        }

        // Request repaint for async operations
        if self.auth_in_progress.is_some() 
            || self.send_in_progress.is_some() 
            || self.received_images_loading
            || self.search_in_progress.is_some()
            || self.recipient_check.is_some()
            || self.username_change_in_progress.is_some()
            || self.poll_in_progress.is_some()
            || self.viewing_in_progress.is_some()
        {
            ctx.request_repaint();
        }
    }
}

impl ClientAppV2 {
    // ========================================================================
    // Auth Page (Login / Register)
    // ========================================================================
    
    fn render_auth_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            
            // Logo / Title
            ui.label(RichText::new("☁️").size(64.0));
            ui.add_space(10.0);
            ui.label(RichText::new("Distributed Image Cloud")
                .size(28.0)
                .color(AppColors::TEXT_PRIMARY)
                .strong());
            ui.label(RichText::new("Secure image sharing with steganography")
                .size(14.0)
                .color(AppColors::TEXT_SECONDARY));
            
            ui.add_space(40.0);
            
            // Auth card
            egui::Frame::default()
                .fill(AppColors::BG_CARD)
                .rounding(Rounding::same(12.0))
                .inner_margin(egui::Margin::same(30.0))
                .show(ui, |ui| {
                    ui.set_width(320.0);
                    
                    // Toggle Login/Register
                    ui.horizontal(|ui| {
                        let login_color = if !self.is_registering { AppColors::PRIMARY } else { AppColors::TEXT_SECONDARY };
                        let register_color = if self.is_registering { AppColors::PRIMARY } else { AppColors::TEXT_SECONDARY };
                        
                        if ui.add(egui::Button::new(RichText::new("Login").color(login_color).size(16.0))
                            .frame(false)).clicked() {
                            self.is_registering = false;
                            self.auth_error = None;
                            self.auth_success = None;
                        }
                        ui.label(RichText::new("|").color(AppColors::BORDER));
                        if ui.add(egui::Button::new(RichText::new("Register").color(register_color).size(16.0))
                            .frame(false)).clicked() {
                            self.is_registering = true;
                            self.auth_error = None;
                            self.auth_success = None;
                        }
                    });
                    
                    ui.add_space(20.0);
                    
                    // Username field
                    ui.label(RichText::new(if self.is_registering { "Choose Username" } else { "Username#ID" })
                        .color(AppColors::TEXT_SECONDARY).size(13.0));
                    ui.add_space(4.0);
                    
                    let username_edit = egui::TextEdit::singleline(&mut self.username_input)
                        .hint_text(if self.is_registering { "Enter username" } else { "e.g., john#42" })
                        .min_size(Vec2::new(280.0, 36.0))
                        .margin(egui::Margin::same(10.0));
                    ui.add(username_edit);
                    
                    ui.add_space(12.0);
                    
                    // Password field
                    ui.label(RichText::new("Password").color(AppColors::TEXT_SECONDARY).size(13.0));
                    ui.add_space(4.0);
                    
                    let password_edit = egui::TextEdit::singleline(&mut self.password_input)
                        .password(true)
                        .hint_text("Enter password")
                        .min_size(Vec2::new(280.0, 36.0))
                        .margin(egui::Margin::same(10.0));
                    ui.add(password_edit);
                    
                    ui.add_space(20.0);
                    
                    // Submit button
                    let button_text = if self.is_registering { "Create Account" } else { "Sign In" };
                    let is_loading = self.auth_in_progress.is_some();
                    let can_submit = !self.username_input.trim().is_empty() 
                        && !self.password_input.is_empty() 
                        && !is_loading;
                    
                    ui.add_enabled_ui(can_submit, |ui| {
                        let button = egui::Button::new(
                            RichText::new(if is_loading { "⏳ Please wait..." } else { button_text })
                                .size(16.0)
                                .color(Color32::WHITE)
                        )
                        .min_size(Vec2::new(280.0, 44.0))
                        .fill(AppColors::PRIMARY)
                        .rounding(Rounding::same(8.0));
                        
                        if ui.add(button).clicked() {
                            self.start_auth(ctx);
                        }
                    });
                    
                    // Show success message (for registration)
                    if let Some(msg) = &self.auth_success {
                        ui.add_space(15.0);
                        egui::Frame::default()
                            .fill(AppColors::SUCCESS.linear_multiply(0.2))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("✓ {}", msg))
                                    .color(AppColors::SUCCESS)
                                    .size(13.0));
                            });
                    }
                    
                    // Show error
                    if let Some(err) = &self.auth_error {
                        ui.add_space(15.0);
                        egui::Frame::default()
                            .fill(AppColors::ERROR.linear_multiply(0.2))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("✗ {}", err))
                                    .color(AppColors::ERROR)
                                    .size(13.0));
                            });
                    }
                });
        });
        
        // Process auth result
        self.process_auth_result();
    }

    fn start_auth(&mut self, _ctx: &egui::Context) {
        let username = self.username_input.trim().to_string();
        let password = self.password_input.clone();
        let is_registering = self.is_registering;
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        self.auth_error = None;
        self.auth_success = None;

        let promise = Promise::spawn_thread("auth", move || {
            runtime.block_on(async move {
                let firebase = FireBaseClient::new();
                
                if is_registering {
                    // REGISTER: Create new user
                    // Generate unique 2-digit ID
                    let mut generated_id = String::new();
                    for _ in 0..10 {
                        let id: u8 = rand::random::<u8>() % 100;
                        let id_str = format!("{:02}", id);
                        match firebase.get_user(&id_str).await {
                            Ok(None) => {
                                generated_id = id_str;
                                break;
                            }
                            _ => continue,
                        }
                    }
                    
                    if generated_id.is_empty() {
                        return Err("Could not generate unique ID".to_string());
                    }
                    
                    let local_ip = get_local_ip();
                    let new_user = UserInfo {
                        id: generated_id.clone(),
                        username: username.clone(),
                        password,
                        status: UserStatus::Online,
                        last_seen: chrono::Utc::now().timestamp(),
                        ip: local_ip,
                        gallery: vec![],
                    };
                    
                    firebase.create_user(&new_user).await
                        .map_err(|e| format!("Failed to create user: {}", e))?;
                    
                    Ok((username, generated_id.clone(), format!("Account created! Your ID is: {}", generated_id)))
                } else {
                    // LOGIN: Verify credentials via node
                    // Parse username#id format
                    if !username.contains('#') {
                        return Err("Please use format: username#id (e.g., john#42)".to_string());
                    }
                    
                    let parts: Vec<&str> = username.rsplitn(2, '#').collect();
                    if parts.len() != 2 {
                        return Err("Invalid format. Use: username#id".to_string());
                    }
                    
                    let user_id = parts[0].to_string();
                    let expected_username = parts[1].to_string();
                    
                    let client = Client::new(0, cloud_addresses);
                    let local_ip = get_local_ip();
                    
                    match client.client_login(user_id.clone(), password, local_ip).await {
                        Ok(Message::ClientLoginResponse { success, user_info, error }) => {
                            if success {
                                if let Some(info) = user_info {
                                    if info.username != expected_username {
                                        return Err("Username doesn't match this ID".to_string());
                                    }
                                    Ok((info.username, user_id, "Login successful".to_string()))
                                } else {
                                    Err("Login succeeded but no user info returned".to_string())
                                }
                            } else {
                                Err(error.unwrap_or_else(|| "Login failed".to_string()))
                            }
                        }
                        Ok(_) => Err("Unexpected response from server".to_string()),
                        Err(e) => {
                            if e.contains("SYSTEM_MAINTENANCE") {
                                Err("🔧 System Under Maintenance".to_string())
                            } else {
                                Err(format!("Connection error: {}", e))
                            }
                        }
                    }
                }
            })
        });

        self.auth_in_progress = Some(promise);
    }

    fn process_auth_result(&mut self) {
        if let Some(promise) = &self.auth_in_progress {
            if let Some(result) = promise.ready() {
                match result {
                    Ok((username, id, message)) => {
                        if self.is_registering {
                            // Show success message with ID, don't auto-login
                            self.auth_success = Some(message.clone());
                            self.is_registering = false;
                            self.username_input = format!("{}#{}", username, id);
                            self.password_input.clear();
                        } else {
                            // Login successful
                            self.session = UserSession {
                                username: username.clone(),
                                user_id: id.clone(),
                                is_logged_in: true,
                            };
                            self.current_page = Page::SendImage;
                            self.username_input.clear();
                            self.password_input.clear();
                            
                            // Sync images from Firebase for cross-device access
                            self.sync_from_firebase();
                            
                            // Load any cached images for this user
                            self.load_cached_images();
                            
                            // Trigger immediate poll for pending images
                            self.last_poll = None;
                        }
                    }
                    Err(e) => {
                        self.auth_error = Some(e.clone());
                    }
                }
                self.auth_in_progress = None;
            }
        }
    }

    // ========================================================================
    // Main App Layout
    // ========================================================================
    
    fn render_main_app(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // Top bar with user info
        self.render_top_bar(ui);
        
        ui.add_space(10.0);
        
        // Navigation tabs
        self.render_navigation(ui);
        
        ui.add_space(20.0);
        
        // Page content
        egui::ScrollArea::vertical().show(ui, |ui| {
            match self.current_page {
                Page::Login => {},  // Shouldn't happen when logged in
                Page::SendImage => self.render_send_image_page(ui, ctx),
                Page::Inbox => self.render_inbox_page(ui, ctx),
                Page::MyGallery => self.render_my_gallery_page(ui, ctx),
                Page::BrowseUsers => self.render_browse_users_page(ui, ctx),
                Page::Settings => self.render_settings_page(ui, ctx),
            }
        });
    }

    fn render_top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .fill(AppColors::BG_CARD)
            .inner_margin(egui::Margin::symmetric(20.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Logo
                    ui.label(RichText::new("☁️").size(24.0));
                    ui.label(RichText::new("Image Cloud")
                        .size(18.0)
                        .color(AppColors::TEXT_PRIMARY)
                        .strong());
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Logout button
                        if ui.add(egui::Button::new(RichText::new("🚪 Logout").size(13.0))
                            .fill(AppColors::ERROR.linear_multiply(0.3))
                            .rounding(Rounding::same(6.0))).clicked() {
                            self.logout();
                        }
                        
                        ui.add_space(15.0);
                        
                        // User info
                        ui.label(RichText::new(format!("{}#{}", self.session.username, self.session.user_id))
                            .size(14.0)
                            .color(AppColors::PRIMARY)
                            .strong());
                        ui.label(RichText::new("👤").size(16.0));
                    });
                });
            });
    }

    fn render_navigation(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            
            let tabs = [
                (Page::SendImage, "📤", "Send Image"),
                (Page::Inbox, "📥", "Inbox"),
                (Page::MyGallery, "🖼️", "My Gallery"),
                (Page::BrowseUsers, "👥", "Browse"),
                (Page::Settings, "⚙️", "Settings"),
            ];
            
            for (page, icon, label) in tabs {
                let is_selected = self.current_page == page;
                let bg_color = if is_selected { AppColors::PRIMARY } else { AppColors::BG_CARD };
                let text_color = if is_selected { Color32::WHITE } else { AppColors::TEXT_SECONDARY };
                
                let button = egui::Button::new(
                    RichText::new(format!("{} {}", icon, label))
                        .size(14.0)
                        .color(text_color)
                )
                .fill(bg_color)
                .rounding(Rounding::same(8.0))
                .min_size(Vec2::new(120.0, 36.0));
                
                if ui.add(button).clicked() {
                    self.current_page = page;
                }
                
                ui.add_space(8.0);
            }
        });
    }

    // ========================================================================
    // Send Image Page
    // ========================================================================
    
    fn render_send_image_page(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() - 40.0);
                
                ui.label(RichText::new("Send Encrypted Image")
                    .size(24.0)
                    .color(AppColors::TEXT_PRIMARY)
                    .strong());
                ui.label(RichText::new("Share images securely with view limits")
                    .size(14.0)
                    .color(AppColors::TEXT_SECONDARY));
                
                ui.add_space(25.0);
                
                // Step 1: Select Image
                self.render_image_selection(ui);
                
                ui.add_space(20.0);
                
                // Step 2: Add Recipients
                self.render_recipient_selection(ui);
                
                ui.add_space(20.0);
                
                // Step 3: View Quota
                self.render_quota_selection(ui);
                
                ui.add_space(20.0);
                
                // Step 4: Send
                self.render_send_button(ui);
            });
        });
    }

    fn render_image_selection(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .fill(AppColors::BG_CARD)
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(20.0))
            .show(ui, |ui| {
                ui.label(RichText::new("1️⃣ Select Image").size(16.0).strong().color(AppColors::TEXT_PRIMARY));
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("📁 Choose File").size(14.0))
                        .fill(AppColors::SECONDARY)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(120.0, 36.0))).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp"])
                            .pick_file() {
                            self.selected_image_path = Some(path.display().to_string());
                        }
                    }
                    
                    ui.add_space(15.0);
                    
                    if let Some(path) = &self.selected_image_path {
                        let filename = std::path::Path::new(path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.clone());
                        ui.label(RichText::new(format!("✓ {}", filename))
                            .color(AppColors::SUCCESS));
                    } else {
                        ui.label(RichText::new("No image selected")
                            .color(AppColors::TEXT_SECONDARY));
                    }
                });
            });
    }

    fn render_recipient_selection(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .fill(AppColors::BG_CARD)
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(20.0))
            .show(ui, |ui| {
                ui.label(RichText::new("2️⃣ Add Recipients").size(16.0).strong().color(AppColors::TEXT_PRIMARY));
                ui.add_space(10.0);
                
                // Add recipient input
                ui.horizontal(|ui| {
                    let text_edit = egui::TextEdit::singleline(&mut self.recipient_input)
                        .hint_text("username#id (e.g., john#42)")
                        .min_size(Vec2::new(200.0, 32.0));
                    ui.add(text_edit);
                    
                    let can_add = !self.recipient_input.trim().is_empty() 
                        && self.recipient_check.is_none();
                    
                    if ui.add_enabled(can_add, egui::Button::new("➕ Add")
                        .fill(AppColors::PRIMARY)
                        .rounding(Rounding::same(6.0))).clicked() {
                        self.check_and_add_recipient();
                    }
                });
                
                // Process recipient check
                self.process_recipient_check();
                
                // Show error
                if let Some(err) = &self.recipient_error {
                    ui.add_space(5.0);
                    ui.label(RichText::new(err).color(AppColors::ERROR).size(12.0));
                }
                
                // Show loading
                if self.recipient_check.is_some() {
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Checking user...").color(AppColors::TEXT_SECONDARY));
                    });
                }
                
                // Show recipients list
                if !self.recipients.is_empty() {
                    ui.add_space(15.0);
                    ui.label(RichText::new("Recipients:").color(AppColors::TEXT_SECONDARY));
                    ui.add_space(5.0);
                    
                    let mut to_remove = None;
                    for (i, recipient) in self.recipients.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("• {}", recipient))
                                .color(AppColors::TEXT_PRIMARY));
                            if ui.small_button("❌").clicked() {
                                to_remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = to_remove {
                        self.recipients.remove(i);
                    }
                }
            });
    }

    fn check_and_add_recipient(&mut self) {
        let recipient = self.recipient_input.trim().to_string();
        
        // Check if already added
        if self.recipients.contains(&recipient) {
            self.recipient_error = Some("User already in list".to_string());
            return;
        }
        
        // Check if it's self
        let self_id = format!("{}#{}", self.session.username, self.session.user_id);
        if recipient == self_id {
            self.recipient_error = Some("Cannot add yourself".to_string());
            return;
        }
        
        self.recipient_error = None;
        
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();
        let recipient_clone = recipient.clone();

        let promise = Promise::spawn_thread("check_recipient", move || {
            let client = Client::new(0, cloud_addresses);
            runtime.block_on(async move {
                client.check_username_available(recipient_clone).await
            })
        });

        self.recipient_check = Some(promise);
    }

    fn process_recipient_check(&mut self) {
        if let Some(promise) = &self.recipient_check {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(is_available) => {
                        if !is_available {
                            // User exists, add to list
                            self.recipients.push(self.recipient_input.trim().to_string());
                            self.recipient_input.clear();
                            self.recipient_error = None;
                        } else {
                            // User doesn't exist
                            self.recipient_error = Some(format!("User '{}' not found", self.recipient_input.trim()));
                        }
                    }
                    Err(e) => {
                        self.recipient_error = Some(format!("Error: {}", e));
                    }
                }
                self.recipient_check = None;
            }
        }
    }

    fn render_quota_selection(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .fill(AppColors::BG_CARD)
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(20.0))
            .show(ui, |ui| {
                ui.label(RichText::new("3️⃣ View Limit").size(16.0).strong().color(AppColors::TEXT_PRIMARY));
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Max views per recipient:")
                        .color(AppColors::TEXT_SECONDARY));
                    
                    ui.add_space(10.0);
                    
                    if ui.button("➖").clicked() && self.view_quota > 1 {
                        self.view_quota -= 1;
                    }
                    
                    ui.label(RichText::new(format!(" {} ", self.view_quota))
                        .size(18.0)
                        .color(AppColors::PRIMARY)
                        .strong());
                    
                    if ui.button("➕").clicked() && self.view_quota < 99 {
                        self.view_quota += 1;
                    }
                });
            });
    }

    fn render_send_button(&mut self, ui: &mut egui::Ui) {
        let can_send = self.selected_image_path.is_some() 
            && !self.recipients.is_empty()
            && self.send_in_progress.is_none();
        
        ui.add_enabled_ui(can_send, |ui| {
            let button = egui::Button::new(
                RichText::new("🚀 Encrypt & Send")
                    .size(18.0)
                    .color(Color32::WHITE)
            )
            .fill(AppColors::SUCCESS)
            .rounding(Rounding::same(10.0))
            .min_size(Vec2::new(200.0, 50.0));
            
            if ui.add(button).clicked() {
                self.send_encrypted_image();
            }
        });
        
        // Show send progress/result
        if self.send_in_progress.is_some() {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Encrypting and sending...").color(AppColors::TEXT_SECONDARY));
            });
            self.process_send_result();
        }
        
        if let Some(result) = &self.send_result {
            ui.add_space(10.0);
            match result {
                Ok(msg) => {
                    ui.label(RichText::new(format!("✓ {}", msg)).color(AppColors::SUCCESS));
                }
                Err(msg) => {
                    ui.label(RichText::new(format!("✗ {}", msg)).color(AppColors::ERROR));
                }
            }
        }
    }

    fn send_encrypted_image(&mut self) {
        let image_path = match &self.selected_image_path {
            Some(p) => p.clone(),
            None => return,
        };
        
        let recipients = self.recipients.clone();
        let quota = self.view_quota;
        let from_user = format!("{}#{}", self.session.username, self.session.user_id);
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();
        
        self.send_result = None;

        let promise = Promise::spawn_thread("send_image", move || {
            // Read image file
            let image_data = match std::fs::read(&image_path) {
                Ok(data) => data,
                Err(e) => return Err(format!("Failed to read image: {}", e)),
            };

            runtime.block_on(async move {
                // Encrypt image
                let encrypted = crate::encryption::encrypt_image(
                    image_data,
                    recipients.clone(),
                    quota,
                ).await.map_err(|e| format!("Encryption failed: {}", e))?;

                // Send to cloud
                let client = Client::new(0, cloud_addresses);
                let image_id = format!("img_{}", chrono::Utc::now().timestamp_millis());
                
                client.send_image(
                    from_user,
                    recipients,
                    encrypted,
                    quota,
                    image_id,
                ).await
            })
        });

        self.send_in_progress = Some(promise);
    }

    fn process_send_result(&mut self) {
        let should_clear = if let Some(promise) = &self.send_in_progress {
            if let Some(result) = promise.ready() {
                self.send_result = Some(result.clone());
                Some(result.is_ok())
            } else {
                None
            }
        } else {
            None
        };
        
        if let Some(clear) = should_clear {
            self.send_in_progress = None;
            if clear {
                self.selected_image_path = None;
                self.recipients.clear();
            }
        }
    }

    // ========================================================================
    // Inbox Page (Images sent TO you)
    // ========================================================================
    
    fn render_inbox_page(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() - 40.0);
                
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📥 Inbox")
                        .size(24.0)
                        .color(AppColors::TEXT_PRIMARY)
                        .strong());
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("🔄 Refresh")
                            .fill(AppColors::SECONDARY)
                            .rounding(Rounding::same(6.0))).clicked() {
                            // Reload from local cache
                            self.load_cached_images();
                        }
                    });
                });
                
                ui.label(RichText::new("Private images shared with you (view quota applies)")
                    .size(14.0)
                    .color(AppColors::TEXT_SECONDARY));
                
                ui.add_space(25.0);
                
                // Show loading if polling is in progress
                if self.poll_in_progress.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Checking for new images...");
                    });
                    ui.add_space(10.0);
                }
                
                if self.received_images.is_empty() {
                    egui::Frame::default()
                        .fill(AppColors::BG_CARD)
                        .rounding(Rounding::same(10.0))
                        .inner_margin(egui::Margin::same(40.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new("📭").size(48.0));
                                ui.add_space(10.0);
                                ui.label(RichText::new("Your inbox is empty")
                                    .size(18.0)
                                    .color(AppColors::TEXT_SECONDARY));
                                ui.label(RichText::new("Images sent to you will appear here")
                                    .color(AppColors::TEXT_SECONDARY));
                            });
                        });
                } else {
                    // Show inbox as a simple grid
                    let available_width = ui.available_width();
                    let card_width = 170.0;
                    let columns = ((available_width / card_width) as usize).max(1);
                    
                    // Clone data to avoid borrow issues
                    let images_data: Vec<(usize, String, u8, u8)> = self.received_images
                        .iter()
                        .enumerate()
                        .map(|(i, img)| (i, img.from_user.clone(), img.remaining_views, img.max_views))
                        .collect();
                    
                    let viewing_in_progress = self.viewing_in_progress.is_some();
                    let mut clicked_view: Option<usize> = None;
                    
                    egui::Grid::new("inbox_grid")
                        .num_columns(columns)
                        .spacing([12.0, 12.0])
                        .show(ui, |ui| {
                            for (i, from_user, remaining_views, max_views) in &images_data {
                                egui::Frame::default()
                                    .fill(AppColors::BG_CARD)
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(egui::Margin::same(10.0))
                                    .show(ui, |ui| {
                                        ui.set_width(150.0);
                                        
                                        // Image placeholder
                                        ui.vertical_centered(|ui| {
                                            ui.label(RichText::new("📨").size(32.0));
                                        });
                                        
                                        ui.add_space(6.0);
                                        
                                        // Sender info
                                        ui.label(RichText::new(format!("From: {}", from_user))
                                            .size(11.0)
                                            .color(AppColors::TEXT_SECONDARY));
                                        
                                        // Views counter
                                        let views_color = if *remaining_views > 0 { 
                                            AppColors::SUCCESS 
                                        } else { 
                                            AppColors::ERROR 
                                        };
                                        ui.label(RichText::new(format!("Views: {}/{}", remaining_views, max_views))
                                            .size(11.0)
                                            .color(views_color));
                                        
                                        ui.add_space(6.0);
                                        
                                        // View button
                                        if ui.add_enabled(
                                            *remaining_views > 0 && !viewing_in_progress,
                                            egui::Button::new("👁 View")
                                                .min_size(Vec2::new(140.0, 24.0))
                                        ).clicked() {
                                            clicked_view = Some(*i);
                                        }
                                    });
                                
                                if (i + 1) % columns == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                    
                    // Handle view click after loop
                    if let Some(idx) = clicked_view {
                        self.view_cached_image(idx);
                    }
                }
            });
        });
    }

    // ========================================================================
    // My Gallery Page (Public pixelated images - max 5)
    // ========================================================================
    
    fn render_my_gallery_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() - 40.0);
                
                ui.label(RichText::new("🖼️ My Gallery")
                    .size(24.0)
                    .color(AppColors::TEXT_PRIMARY)
                    .strong());
                
                ui.label(RichText::new("Share up to 5 pixelated preview images publicly")
                    .size(14.0)
                    .color(AppColors::TEXT_SECONDARY));
                
                ui.add_space(20.0);
                
                // Load gallery if not loaded yet
                if !self.my_gallery_loaded && self.my_gallery_loading.is_none() {
                    self.load_my_gallery();
                }
                
                // Process loading result
                self.process_my_gallery_load();
                
                // Show loading spinner
                if self.my_gallery_loading.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading your gallery...");
                    });
                    return;
                }
                
                // Show error if any
                if let Some(error) = &self.gallery_error {
                    ui.label(RichText::new(error).color(AppColors::ERROR));
                    ui.add_space(10.0);
                }
                
                // Gallery count
                ui.label(RichText::new(format!("Images: {}/5", self.my_gallery.len()))
                    .size(14.0)
                    .color(AppColors::TEXT_SECONDARY));
                ui.add_space(15.0);
                
                // Gallery grid
                if self.my_gallery.is_empty() {
                    egui::Frame::default()
                        .fill(AppColors::BG_CARD)
                        .rounding(Rounding::same(10.0))
                        .inner_margin(egui::Margin::same(40.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new("🖼️").size(48.0));
                                ui.add_space(10.0);
                                ui.label(RichText::new("Your gallery is empty")
                                    .size(18.0)
                                    .color(AppColors::TEXT_SECONDARY));
                                ui.label(RichText::new("Add up to 5 images for others to preview")
                                    .color(AppColors::TEXT_SECONDARY));
                            });
                        });
                } else {
                    // Clone gallery data to avoid borrow issues
                    let gallery_data: Vec<String> = self.my_gallery.clone();
                    let num_images = gallery_data.len();
                    
                    // Ensure textures vec is correct size
                    if self.my_gallery_textures.len() != num_images {
                        self.my_gallery_textures = vec![None; num_images];
                    }
                    
                    egui::Grid::new("my_gallery_grid")
                        .num_columns(5)
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            let mut remove_index: Option<usize> = None;
                            
                            for (i, data_url) in gallery_data.iter().enumerate() {
                                egui::Frame::default()
                                    .fill(AppColors::BG_CARD)
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(egui::Margin::same(8.0))
                                    .show(ui, |ui| {
                                        ui.set_width(100.0);
                                        
                                        // Load and display texture if not loaded
                                        if i < self.my_gallery_textures.len() {
                                            if self.my_gallery_textures[i].is_none() {
                                                // Try to load from data URL
                                                if let Some(texture) = load_texture_from_data_url(ctx, data_url, i) {
                                                    self.my_gallery_textures[i] = Some(texture);
                                                }
                                            }
                                            
                                            if let Some(texture) = &self.my_gallery_textures[i] {
                                                ui.add(egui::Image::new(texture).max_size(Vec2::new(80.0, 80.0)));
                                            } else {
                                                ui.add_sized([80.0, 80.0], egui::Label::new(
                                                    RichText::new("🖼️").size(30.0)
                                                ));
                                            }
                                        }
                                        
                                        if ui.add(egui::Button::new("🗑️").small()).clicked() {
                                            remove_index = Some(i);
                                        }
                                    });
                            }
                            
                            // Handle removal
                            if let Some(idx) = remove_index {
                                self.remove_from_gallery(idx);
                            }
                        });
                }
                
                ui.add_space(20.0);
                
                // Add image button (only if < 5 images)
                if self.my_gallery.len() < 5 {
                    if self.gallery_upload_in_progress.is_some() {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Uploading...");
                        });
                    } else {
                        if ui.add(egui::Button::new("➕ Add Image to Gallery")
                            .fill(AppColors::PRIMARY)
                            .rounding(Rounding::same(6.0))).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp"])
                                .pick_file() {
                                self.upload_to_gallery(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            });
        });
    }

    fn load_my_gallery(&mut self) {
        let user_id = self.session.user_id.clone();
        let cloud_addresses = self.cloud_addresses.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("load_my_gallery", move || {
            runtime.block_on(async move {
                let client = Client::new(0, cloud_addresses);
                client.get_user_gallery(user_id).await
            })
        });

        self.my_gallery_loading = Some(promise);
    }

    fn process_my_gallery_load(&mut self) {
        let result = if let Some(promise) = &self.my_gallery_loading {
            promise.ready().cloned()
        } else {
            None
        };

        if let Some(res) = result {
            match res {
                Ok(gallery) => {
                    self.my_gallery_textures = vec![None; gallery.len()];
                    self.my_gallery = gallery;
                    self.gallery_error = None;
                }
                Err(e) => {
                    self.gallery_error = Some(e);
                }
            }
            self.my_gallery_loaded = true;
            self.my_gallery_loading = None;
        }
    }

    fn upload_to_gallery(&mut self, path: String) {
        let user_id = self.session.user_id.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();
        let mut current_gallery = self.my_gallery.clone();

        let promise = Promise::spawn_thread("upload_gallery", move || {
            // Read and pixelate the image
            let img = image::open(&path)
                .map_err(|e| format!("Failed to open image: {}", e))?;

            // Pixelate by reducing resolution to 64x64
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
        let user_id = self.session.user_id.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();
        let mut current_gallery = self.my_gallery.clone();

        if index < current_gallery.len() {
            current_gallery.remove(index);
        }

        let promise = Promise::spawn_thread("remove_from_gallery", move || {
            let firebase = FireBaseClient::new();
            runtime.block_on(async move {
                firebase.update_gallery(&user_id, &current_gallery).await
                    .map_err(|e| format!("Failed to update gallery: {}", e))
            })
        });

        // Update local state immediately
        if index < self.my_gallery.len() {
            self.my_gallery.remove(index);
        }
        if index < self.my_gallery_textures.len() {
            self.my_gallery_textures.remove(index);
        }

        self.gallery_upload_in_progress = Some(promise);
    }

    // ========================================================================
    // Browse Users Page
    // ========================================================================
    
    fn render_browse_users_page(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() - 40.0);
                
                ui.label(RichText::new("Browse Users")
                    .size(24.0)
                    .color(AppColors::TEXT_PRIMARY)
                    .strong());
                ui.label(RichText::new("Find users to share images with")
                    .size(14.0)
                    .color(AppColors::TEXT_SECONDARY));
                
                ui.add_space(25.0);
                
                // Search box
                egui::Frame::default()
                    .fill(AppColors::BG_CARD)
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(20.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let search_edit = egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text("🔍 Search by username or ID...")
                                .min_size(Vec2::new(300.0, 36.0));
                            ui.add(search_edit);
                            
                            if ui.add(egui::Button::new("Search")
                                .fill(AppColors::PRIMARY)
                                .rounding(Rounding::same(6.0))).clicked() {
                                self.search_users();
                            }
                        });
                    });
                
                ui.add_space(20.0);
                
                // Process search
                self.process_search_result();
                
                // Show loading
                if self.search_in_progress.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Searching...");
                    });
                }
                
                // Show results
                if !self.search_results.is_empty() {
                    // Clone results to avoid borrow issues
                    let results_clone: Vec<UserSearchResult> = self.search_results.clone();
                    let mut add_recipient: Option<String> = None;
                    
                    for user in &results_clone {
                        egui::Frame::default()
                            .fill(AppColors::BG_CARD)
                            .rounding(Rounding::same(8.0))
                            .inner_margin(egui::Margin::same(15.0))
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        // Status indicator
                                        let status_color = match user.status.as_str() {
                                            "Online" => AppColors::SUCCESS,
                                            "Idle" => AppColors::WARNING,
                                            _ => AppColors::TEXT_SECONDARY,
                                        };
                                        ui.label(RichText::new("●").color(status_color));
                                        
                                        ui.label(RichText::new(format!("{}#{}", user.username, user.user_id))
                                            .size(15.0)
                                            .color(AppColors::TEXT_PRIMARY)
                                            .strong());
                                        
                                        ui.label(RichText::new(format!("({})", user.status))
                                            .size(12.0)
                                            .color(AppColors::TEXT_SECONDARY));
                                        
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let user_tag = format!("{}#{}", user.username, user.user_id);
                                            if ui.small_button("➕ Add to Send").clicked() {
                                                add_recipient = Some(user_tag);
                                            }
                                        });
                                    });
                                    
                                    // Show gallery preview if user has gallery images
                                    if !user.gallery.is_empty() {
                                        ui.add_space(10.0);
                                        ui.label(RichText::new(format!("📷 Gallery ({} images)", user.gallery.len()))
                                            .size(12.0)
                                            .color(AppColors::TEXT_SECONDARY));
                                        ui.add_space(5.0);
                                        
                                        ui.horizontal(|ui| {
                                            // Load/create textures for this user's gallery if not already cached
                                            let user_id = user.user_id.clone();
                                            let gallery = user.gallery.clone();
                                            
                                            if !self.search_gallery_textures.contains_key(&user_id) {
                                                // Load textures from base64 data
                                                let mut textures = Vec::new();
                                                for (idx, data_url) in gallery.iter().enumerate() {
                                                    if let Some(texture) = load_texture_from_data_url_with_name(ui.ctx(), data_url, &format!("search_gallery_{}_{}", user_id, idx)) {
                                                        textures.push(texture);
                                                    }
                                                }
                                                self.search_gallery_textures.insert(user_id.clone(), textures);
                                            }
                                            
                                            // Display cached textures
                                            if let Some(textures) = self.search_gallery_textures.get(&user_id) {
                                                for texture in textures.iter().take(5) {
                                                    let size = Vec2::new(60.0, 60.0);
                                                    let image = egui::Image::new(texture)
                                                        .fit_to_exact_size(size)
                                                        .rounding(Rounding::same(4.0));
                                                    ui.add(image);
                                                }
                                            }
                                        });
                                    }
                                });
                            });
                        ui.add_space(8.0);
                    }
                    
                    // Apply deferred action
                    if let Some(user_tag) = add_recipient {
                        if !self.recipients.contains(&user_tag) {
                            self.recipients.push(user_tag);
                        }
                    }
                }
            });
        });
    }

    fn search_users(&mut self) {
        let query = self.search_query.trim().to_string();
        let runtime = self.runtime.as_ref().unwrap().clone();

        let promise = Promise::spawn_thread("search_users", move || {
            runtime.block_on(async move {
                let firebase = FireBaseClient::new();
                
                // Search by username
                let mut results = Vec::new();
                
                if let Ok(Some((id, info))) = firebase.find_user_by_username(&query).await {
                    // Get gallery for this user
                    let gallery = firebase.get_user_gallery(&id).await.unwrap_or_default();
                    results.push(UserSearchResult {
                        username: info.username,
                        user_id: id,
                        status: format!("{:?}", info.status),
                        last_seen: info.last_seen,
                        gallery,
                    });
                }
                
                // Also try searching by ID
                if let Ok(Some(info)) = firebase.get_user(&query).await {
                    let already_added = results.iter().any(|r| r.user_id == query);
                    if !already_added {
                        // Get gallery for this user
                        let gallery = firebase.get_user_gallery(&query).await.unwrap_or_default();
                        results.push(UserSearchResult {
                            username: info.username,
                            user_id: query.clone(),
                            status: format!("{:?}", info.status),
                            last_seen: info.last_seen,
                            gallery,
                        });
                    }
                }
                
                Ok(results)
            })
        });

        self.search_in_progress = Some(promise);
    }

    fn process_search_result(&mut self) {
        if let Some(promise) = &self.search_in_progress {
            if let Some(result) = promise.ready() {
                if let Ok(results) = result {
                    self.search_results = results.clone();
                }
                self.search_in_progress = None;
            }
        }
    }

    // ========================================================================
    // Settings Page
    // ========================================================================
    
    fn render_settings_page(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            
            ui.vertical(|ui| {
                ui.set_width(ui.available_width() - 40.0);
                
                ui.label(RichText::new("Settings")
                    .size(24.0)
                    .color(AppColors::TEXT_PRIMARY)
                    .strong());
                ui.label(RichText::new("Manage your account")
                    .size(14.0)
                    .color(AppColors::TEXT_SECONDARY));
                
                ui.add_space(25.0);
                
                // Account info
                egui::Frame::default()
                    .fill(AppColors::BG_CARD)
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(20.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Account Information").size(16.0).strong().color(AppColors::TEXT_PRIMARY));
                        ui.add_space(15.0);
                        
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Username:").color(AppColors::TEXT_SECONDARY));
                            ui.label(RichText::new(&self.session.username).color(AppColors::TEXT_PRIMARY).strong());
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("ID:").color(AppColors::TEXT_SECONDARY));
                            ui.label(RichText::new(&self.session.user_id).color(AppColors::PRIMARY).strong());
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Full Tag:").color(AppColors::TEXT_SECONDARY));
                            ui.label(RichText::new(format!("{}#{}", self.session.username, self.session.user_id))
                                .color(AppColors::TEXT_PRIMARY));
                        });
                    });
                
                ui.add_space(20.0);
                
                // Change username
                egui::Frame::default()
                    .fill(AppColors::BG_CARD)
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(20.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Change Username").size(16.0).strong().color(AppColors::TEXT_PRIMARY));
                        ui.add_space(15.0);
                        
                        ui.horizontal(|ui| {
                            let text_edit = egui::TextEdit::singleline(&mut self.new_username_input)
                                .hint_text("New username")
                                .min_size(Vec2::new(200.0, 32.0));
                            ui.add(text_edit);
                            
                            let can_change = !self.new_username_input.trim().is_empty()
                                && self.new_username_input.trim() != self.session.username
                                && self.username_change_in_progress.is_none();
                            
                            if ui.add_enabled(can_change, egui::Button::new("Update")
                                .fill(AppColors::PRIMARY)
                                .rounding(Rounding::same(6.0))).clicked() {
                                self.change_username();
                            }
                        });
                        
                        // Process change
                        self.process_username_change();
                        
                        // Show loading
                        if self.username_change_in_progress.is_some() {
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Updating...");
                            });
                        }
                        
                        // Show message
                        if let Some((msg, is_error)) = &self.settings_message {
                            ui.add_space(10.0);
                            let color = if *is_error { AppColors::ERROR } else { AppColors::SUCCESS };
                            ui.label(RichText::new(msg).color(color));
                        }
                    });
                
                ui.add_space(20.0);
                
                // Connected nodes info
                egui::Frame::default()
                    .fill(AppColors::BG_CARD)
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(20.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Connected Nodes").size(16.0).strong().color(AppColors::TEXT_PRIMARY));
                        ui.add_space(10.0);
                        
                        for addr in &self.cloud_addresses {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("●").color(AppColors::SUCCESS));
                                ui.label(RichText::new(addr).color(AppColors::TEXT_SECONDARY));
                            });
                        }
                    });
            });
        });
    }

    fn change_username(&mut self) {
        let new_username = self.new_username_input.trim().to_string();
        let user_id = self.session.user_id.clone();
        let runtime = self.runtime.as_ref().unwrap().clone();

        self.settings_message = None;

        let promise = Promise::spawn_thread("change_username", move || {
            runtime.block_on(async move {
                let firebase = FireBaseClient::new();
                firebase.update_user_name(&user_id, &new_username).await
                    .map_err(|e| format!("Failed to update: {}", e))
            })
        });

        self.username_change_in_progress = Some(promise);
    }

    fn process_username_change(&mut self) {
        if let Some(promise) = &self.username_change_in_progress {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(()) => {
                        self.session.username = self.new_username_input.trim().to_string();
                        self.settings_message = Some(("Username updated successfully!".to_string(), false));
                        self.new_username_input.clear();
                    }
                    Err(e) => {
                        self.settings_message = Some((e.clone(), true));
                    }
                }
                self.username_change_in_progress = None;
            }
        }
    }

    // ========================================================================
    // Image Viewer Popup
    // ========================================================================
    
    fn render_image_viewer(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();
        
        // Gather data to avoid borrow issues
        let is_loading = self.viewing_in_progress.is_some();
        let error_msg = self.view_error.clone();
        let texture = self.viewing_image_texture.clone();
        let image_info = self.selected_received_image
            .and_then(|idx| self.received_images.get(idx))
            .map(|img| (img.from_user.clone(), img.remaining_views));
        
        let mut should_close = false;
        
        egui::Window::new("Image Viewer")
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_size([screen_rect.width() * 0.7, screen_rect.height() * 0.8])
            .show(ctx, |ui| {
                // Loading state
                if is_loading {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.spinner();
                        ui.add_space(10.0);
                        ui.label("Decrypting image...");
                    });
                    return;
                }
                
                // Error state
                if let Some(error) = &error_msg {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new("❌ Error").size(24.0).color(AppColors::ERROR));
                        ui.add_space(10.0);
                        ui.label(RichText::new(error).color(AppColors::ERROR));
                        ui.add_space(20.0);
                        if ui.button("Close").clicked() {
                            should_close = true;
                        }
                    });
                    return;
                }
                
                // Show the image
                if let Some(tex) = &texture {
                    ui.vertical_centered(|ui| {
                        // Header with info
                        if let Some((from_user, remaining_views)) = &image_info {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("From: {}", from_user))
                                    .size(14.0)
                                    .color(AppColors::TEXT_SECONDARY));
                                ui.separator();
                                let views_color = if *remaining_views > 0 { AppColors::SUCCESS } else { AppColors::WARNING };
                                ui.label(RichText::new(format!("Views remaining: {}", remaining_views))
                                    .size(14.0)
                                    .color(views_color));
                            });
                            ui.add_space(10.0);
                        }
                        
                        // Display the image, scaled to fit
                        let available = ui.available_size();
                        let tex_size = tex.size_vec2();
                        let scale = (available.x / tex_size.x).min(available.y / tex_size.y).min(1.0);
                        let display_size = tex_size * scale * 0.9;
                        
                        ui.add(egui::Image::new(tex).max_size(display_size));
                        
                        ui.add_space(15.0);
                        
                        // Close button
                        if ui.add(egui::Button::new(RichText::new("✖ Close").size(14.0))
                            .min_size(Vec2::new(100.0, 30.0))
                            .fill(AppColors::ERROR)).clicked() {
                            should_close = true;
                        }
                    });
                }
            });
        
        if should_close {
            self.close_image_viewer();
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Load a texture from a base64 data URL
fn load_texture_from_data_url(ctx: &egui::Context, data_url: &str, idx: usize) -> Option<egui::TextureHandle> {
    load_texture_from_data_url_with_name(ctx, data_url, &format!("gallery_{}", idx))
}

/// Load a texture from a base64 data URL with a custom name
fn load_texture_from_data_url_with_name(ctx: &egui::Context, data_url: &str, name: &str) -> Option<egui::TextureHandle> {
    // Parse data URL: data:image/png;base64,<data>
    if let Some(base64_data) = data_url.strip_prefix("data:image/png;base64,") {
        if let Ok(bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data) {
            if let Ok(img) = image::load_from_memory(&bytes) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                return Some(ctx.load_texture(name.to_string(), color_image, egui::TextureOptions::default()));
            }
        }
    }
    None
}

fn get_local_ip() -> String {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok();
    if let Some(s) = socket {
        if s.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = s.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}
