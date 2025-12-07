use serde::{Deserialize, Serialize, Deserializer};
use std::collections::HashMap;

pub struct FireBaseClient {
    client: reqwest::Client,
    base_url: String,
}

impl Default for FireBaseClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom deserializer that handles empty string as empty vec
fn deserialize_gallery<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    
    struct GalleryVisitor;
    
    impl<'de> Visitor<'de> for GalleryVisitor {
        type Value = Vec<String>;
        
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sequence or empty string")
        }
        
        fn visit_str<E>(self, _v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            // Empty string means empty gallery
            Ok(vec![])
        }
        
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(value) = seq.next_element()? {
                vec.push(value);
            }
            Ok(vec)
        }
        
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![])
        }
    }
    
    deserializer.deserialize_any(GalleryVisitor)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UserStatus { Online, Offline, Idle }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    #[serde(default)]
    pub id: String,
    pub username: String,
    pub password: String,
    pub status: UserStatus,
    pub last_seen: i64,
    #[serde(default)]
    pub ip: String,
    #[serde(deserialize_with = "deserialize_gallery", default)]
    pub gallery: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NodeStatus { Active, Inactive }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeInfo {
    pub address: String,
    #[serde(deserialize_with = "deserialize_node_status")]
    pub status: NodeStatus,
}

/// Custom deserializer to handle various status string formats
fn deserialize_node_status<'de, D>(deserializer: D) -> Result<NodeStatus, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    
    struct NodeStatusVisitor;
    
    impl<'de> Visitor<'de> for NodeStatusVisitor {
        type Value = NodeStatus;
        
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string like 'Active', 'online', 'Inactive', 'offline'")
        }
        
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v.to_lowercase().as_str() {
                "active" | "online" => Ok(NodeStatus::Active),
                "inactive" | "offline" => Ok(NodeStatus::Inactive),
                _ => Ok(NodeStatus::Inactive), // Default to inactive for unknown
            }
        }
    }
    
    deserializer.deserialize_str(NodeStatusVisitor)
}

impl FireBaseClient {
    /// Create a new FireBaseClient with the given base URL
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://dist-b6621-default-rtdb.europe-west1.firebasedatabase.app".to_string(),
        }
    }


    pub async fn create_user(&self, info: &UserInfo) -> Result<(), reqwest::Error>
    {
        let url = format!("{}/users/{}.json", self.base_url, info.id);
        self.client.put(&url).json(info).send().await?;
        Ok(())
    }
    // ==================== USER GETTERS ====================


    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<(String, UserInfo)>, reqwest::Error>
    {
        let url = format!("{}/users.json?orderBy=\"username\"&equalTo=\"{}\"", self.base_url, username);
        let resp = self.client.get(&url).send().await?;
        let users_map = resp.json::<Option<std::collections::HashMap<String, UserInfo>>>().await?;
        if let Some(users) = users_map {
            for (id, info) in users {
                return Ok(Some((id, info)));
            }
        }
        Ok(None)
    }

    /// Find all users that match the given username (returns Vec of (id, UserInfo)).
    pub async fn find_users_by_username(&self, username: &str) -> Result<Vec<(String, UserInfo)>, reqwest::Error>
    {
        let url = format!("{}/users.json?orderBy=\"username\"&equalTo=\"{}\"", self.base_url, username);
        let resp = self.client.get(&url).send().await?;
        let users_map = resp.json::<Option<std::collections::HashMap<String, UserInfo>>>().await?;
        let mut results = Vec::new();
        if let Some(users) = users_map {
            for (id, info) in users {
                results.push((id, info));
            }
        }
        Ok(results)
    }

    /// Get user by ID, returns None if user doesn't exist
    pub async fn get_user(&self, id: &str) -> Result<Option<UserInfo>, reqwest::Error> {
        let url = format!("{}/users/{}.json", self.base_url, id);
        let resp = self.client.get(&url).send().await?;
        let user_info = resp.json::<Option<UserInfo>>().await?;
        Ok(user_info)
    }

    pub async fn get_user_info(&self, id: &str) -> Result<UserInfo, reqwest::Error> {
        let url = format!("{}/users/{}.json", self.base_url, id);
        let resp = self.client.get(&url).send().await?;
        let user_info = resp.json::<UserInfo>().await?;
        Ok(user_info)
    }

    pub async fn get_user_name(&self, id: &str) -> Result<Option<String>, reqwest::Error> {
        let url = format!("{}/users/{}/username.json", self.base_url, id);
        let resp = self.client.get(&url).send().await?;
        let username = resp.json::<Option<String>>().await?;
        Ok(username)
    }

    pub async fn get_user_gallery(&self, id: &str) -> Result<Vec<String>, reqwest::Error> {
        let url = format!("{}/users/{}/gallery.json", self.base_url, id);
        let resp = self.client.get(&url).send().await?;
        let text = resp.text().await?;
        
        // Handle empty string, null, or missing gallery
        if text.is_empty() || text == "null" || text == "\"\"" {
            return Ok(vec![]);
        }
        
        // Try to parse as array
        match serde_json::from_str::<Vec<String>>(&text) {
            Ok(gallery) => Ok(gallery),
            Err(_) => Ok(vec![]),  // If parsing fails, return empty gallery
        }
    }

    pub async fn get_user_status(&self, id: &str) -> Result<UserStatus, reqwest::Error> {
        let url = format!("{}/users/{}/status.json", self.base_url, id);
        let resp = self.client.get(&url).send().await?;
        let user_status = resp.json::<UserStatus>().await?;
        Ok(user_status)
    }

    pub async fn get_user_last_seen(&self, id: &str) -> Result<i64, reqwest::Error> {
        let url = format!("{}/users/{}/last_seen.json", self.base_url, id);
        let resp = self.client.get(&url).send().await?;
        let last_seen = resp.json::<i64>().await?;
        Ok(last_seen)
    }

    pub async fn get_user_ip(&self, id: &str) -> Result<Option<String>, reqwest::Error> {
        let url = format!("{}/users/{}/ip.json", self.base_url, id);
        let resp = self.client.get(&url).send().await?;
        let ip = resp.json::<Option<String>>().await?;
        Ok(ip)
    }

    /// Check if the provided password matches the stored password
    pub async fn verify_password(&self, id: &str, password: &str) -> Result<bool, reqwest::Error> {
        let url = format!("{}/users/{}/password.json", self.base_url, id);
        let resp = self.client.get(&url).send().await?;
        let stored_password = resp.json::<Option<String>>().await?;
        Ok(stored_password.map(|p| p == password).unwrap_or(false))
    }


    // ==================== USER SETTERS ====================

    pub async fn update_user_info(&self, id: &str, info: &UserInfo) -> Result<(), reqwest::Error> {
        let url = format!("{}/users/{}.json", self.base_url, id);
        self.client.put(&url).json(info).send().await?;
        Ok(())
    }

    pub async fn update_user_status(&self, id: &str, status: &UserStatus) -> Result<(), reqwest::Error> {
        let url = format!("{}/users/{}/status.json", self.base_url, id);
        self.client.put(&url).json(status).send().await?;
        Ok(())
    }

    pub async fn update_user_name(&self, id: &str, username: &str) -> Result<(), reqwest::Error> {
        let url = format!("{}/users/{}/username.json", self.base_url, id);
        self.client.put(&url).json(username).send().await?;
        Ok(())
    }

    pub async fn update_password(&self, id: &str, password: &str) -> Result<(), reqwest::Error> {
        let url = format!("{}/users/{}/password.json", self.base_url, id);
        self.client.put(&url).json(password).send().await?;
        Ok(())
    }

    pub async fn update_last_seen(&self, id: &str, timestamp: i64) -> Result<(), reqwest::Error> {
        let url = format!("{}/users/{}/last_seen.json", self.base_url, id);
        self.client.put(&url).json(&timestamp).send().await?;
        Ok(())
    }

    pub async fn update_user_ip(&self, id: &str, ip: &str) -> Result<(), reqwest::Error> {
        let url = format!("{}/users/{}/ip.json", self.base_url, id);
        self.client.put(&url).json(ip).send().await?;
        Ok(())
    }

    pub async fn update_gallery(&self, id: &str, gallery: &Vec<String>) -> Result<(), reqwest::Error> {
        let url = format!("{}/users/{}/gallery.json", self.base_url, id);
        self.client.put(&url).json(gallery).send().await?;
        Ok(())
    }
    
    /// Update full resolution gallery (stores original images for sending when requested)
    pub async fn update_full_gallery(&self, id: &str, full_gallery: &Vec<String>) -> Result<(), reqwest::Error> {
        let url = format!("{}/users/{}/full_gallery.json", self.base_url, id);
        self.client.put(&url).json(full_gallery).send().await?;
        Ok(())
    }
    
    /// Get full resolution gallery
    pub async fn get_full_gallery(&self, id: &str) -> Result<Vec<String>, reqwest::Error> {
        let url = format!("{}/users/{}/full_gallery.json", self.base_url, id);
        let resp = self.client.get(&url).send().await?;
        let gallery = resp.json::<Option<Vec<String>>>().await?;
        Ok(gallery.unwrap_or_default())
    }

    pub async fn get_full_gallery_image(&self, id: &str, index: usize) -> Result<String, String> {
        let url = format!("{}/users/{}/full_gallery/{}.json", self.base_url, id, index);
        let resp = self.client.get(&url).send().await
            .map_err(|e| format!("HTTP error: {}", e))?;
        let image = resp.json::<Option<String>>().await
            .map_err(|e| format!("JSON parse error: {}", e))?;
        image.ok_or_else(|| format!("Image at index {} not found", index))
    }

    pub async fn add_to_gallery(&self, id: &str, image_id: &str) -> Result<(), reqwest::Error> {
        let url = format!("{}/users/{}/gallery.json", self.base_url, id);
        self.client.post(&url).json(image_id).send().await?;
        Ok(())
    }

    // ==================== NODE OPERATIONS ====================

    pub async fn get_all_nodes(&self) -> Result<HashMap<String, NodeInfo>, reqwest::Error>{
        let url = format!("{}/nodes.json", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let nodes_map = resp.json::<Option<HashMap<String, NodeInfo>>>().await?;
        Ok(nodes_map.unwrap_or_default())
    }
    pub async fn get_node_info(&self, node_id: u8) -> Result<NodeInfo, reqwest::Error> {
        let url = format!("{}/nodes/node{}.json", self.base_url, node_id);
        let resp = self.client.get(&url).send().await?;
        let node_info = resp.json::<NodeInfo>().await?;
        Ok(node_info)
    }

    pub async fn update_node_info(&self, node_id: u8, info: &NodeInfo) -> Result<(), reqwest::Error> {
        let url = format!("{}/nodes/node{}.json", self.base_url, node_id);
        self.client.put(&url).json(info).send().await?;
        Ok(())
    }

    pub async fn update_node_status(&self, node_id: u8, status: &NodeStatus) -> Result<(), reqwest::Error> {
        let url = format!("{}/nodes/node{}/status.json", self.base_url, node_id);
        self.client.put(&url).json(status).send().await?;
        Ok(())
    }

    pub async fn update_node_address(&self, node_id: u8, address: &str) -> Result<(), reqwest::Error> {
        let url = format!("{}/nodes/node{}/address.json", self.base_url, node_id);
        self.client.put(&url).json(address).send().await?;
        Ok(())
    }

    pub async fn get_node_address(&self, node_id: u8) -> Result<Option<String>, reqwest::Error> {
        let url = format!("{}/nodes/node{}/address.json", self.base_url, node_id);
        let resp = self.client.get(&url).send().await?;
        let address = resp.json::<Option<String>>().await?;
        Ok(address)
    }

    /// Get all online node addresses from Firebase
    pub async fn get_online_node_addresses(&self) -> Result<Vec<String>, reqwest::Error> {
        let nodes = self.get_all_nodes().await?;
        let mut addresses: Vec<String> = nodes
            .into_iter()
            .filter(|(_, info)| {
                // Include nodes that are Active
                matches!(info.status, NodeStatus::Active)
            })
            .filter_map(|(_, info)| {
                if !info.address.is_empty() && info.address != "127.0.0.1:0" {
                    Some(info.address)
                } else {
                    None
                }
            })
            .collect();
        
        // Sort for consistent ordering
        addresses.sort();
        Ok(addresses)
    }

    /// Check if a username is available (not registered by anyone)
    /// Returns true if available (nobody has this username), false if taken
    pub async fn check_username_available(&self, username: &str) -> Result<bool, reqwest::Error> {
        match self.find_user_by_username(username).await? {
            Some(_) => Ok(false),  // Username is taken
            None => Ok(true),      // Username is available
        }
    }

    // ==================== RECEIVED IMAGES OPERATIONS ====================

    /// Store received image metadata in Firebase for a user
    pub async fn add_received_image(&self, user_id: &str, image_meta: &ReceivedImageMeta) -> Result<(), reqwest::Error> {
        let url = format!("{}/received_images/{}/{}.json", self.base_url, user_id, image_meta.image_id);
        self.client.put(&url).json(image_meta).send().await?;
        Ok(())
    }

    /// Get all received images for a user from Firebase
    pub async fn get_received_images(&self, user_id: &str) -> Result<Vec<ReceivedImageMeta>, reqwest::Error> {
        let url = format!("{}/received_images/{}.json", self.base_url, user_id);
        let resp = self.client.get(&url).send().await?;
        let images_map = resp.json::<Option<HashMap<String, ReceivedImageMeta>>>().await?;
        
        match images_map {
            Some(map) => Ok(map.into_values().collect()),
            None => Ok(vec![]),
        }
    }

    /// Get a specific received image for a user from Firebase
    pub async fn get_received_image(&self, user_id: &str, image_id: &str) -> Result<Option<ReceivedImageMeta>, reqwest::Error> {
        let url = format!("{}/received_images/{}/{}.json", self.base_url, user_id, image_id);
        let resp = self.client.get(&url).send().await?;
        let image_meta = resp.json::<Option<ReceivedImageMeta>>().await?;
        Ok(image_meta)
    }

    /// Update view count for a received image
    pub async fn update_received_image_views(&self, user_id: &str, image_id: &str, remaining_views: u8) -> Result<(), reqwest::Error> {
        let url = format!("{}/received_images/{}/{}/remaining_views.json", self.base_url, user_id, image_id);
        self.client.put(&url).json(&remaining_views).send().await?;
        Ok(())
    }

    /// Delete a received image record (when views exhausted)
    pub async fn delete_received_image(&self, user_id: &str, image_id: &str) -> Result<(), reqwest::Error> {
        let url = format!("{}/received_images/{}/{}.json", self.base_url, user_id, image_id);
        self.client.delete(&url).send().await?;
        Ok(())
    }

    // ==================== NOTES OPERATIONS ====================

    /// Store a note in Firebase for a user
    pub async fn add_note(&self, user_id: &str, note: &NoteMeta) -> Result<(), reqwest::Error> {
        let url = format!("{}/notes/{}/{}.json", self.base_url, user_id, note.note_id);
        self.client.put(&url).json(note).send().await?;
        Ok(())
    }

    /// Get all notes for a user from Firebase
    pub async fn get_notes(&self, user_id: &str) -> Result<Vec<NoteMeta>, reqwest::Error> {
        let url = format!("{}/notes/{}.json", self.base_url, user_id);
        let resp = self.client.get(&url).send().await?;
        let notes_map = resp.json::<Option<HashMap<String, NoteMeta>>>().await?;
        
        match notes_map {
            Some(map) => Ok(map.into_values().collect()),
            None => Ok(vec![]),
        }
    }

    /// Delete a note from Firebase
    pub async fn delete_note(&self, user_id: &str, note_id: &str) -> Result<(), reqwest::Error> {
        let url = format!("{}/notes/{}/{}.json", self.base_url, user_id, note_id);
        self.client.delete(&url).send().await?;
        Ok(())
    }
    
    // ==================== IMAGE REQUESTS OPERATIONS ====================
    
    /// Store an image request in Firebase
    pub async fn add_image_request(&self, request: &ImageRequestMeta) -> Result<(), reqwest::Error> {
        // Store in both users' request lists in parallel for better performance
        let url_from = format!("{}/image_requests/{}/outgoing/{}.json", 
            self.base_url, request.from_user_id, request.request_id);
        let url_to = format!("{}/image_requests/{}/incoming/{}.json", 
            self.base_url, request.to_user_id, request.request_id);
        
        // Execute both writes in parallel
        let (result_from, result_to) = tokio::join!(
            self.client.put(&url_from).json(request).send(),
            self.client.put(&url_to).json(request).send()
        );
        
        result_from?;
        result_to?;
        
        Ok(())
    }
    
    /// Get all incoming image requests for a user
    pub async fn get_incoming_image_requests(&self, user_id: &str) -> Result<Vec<ImageRequestMeta>, reqwest::Error> {
        let url = format!("{}/image_requests/{}/incoming.json", self.base_url, user_id);
        let resp = self.client.get(&url).send().await?;
        let requests_map = resp.json::<Option<HashMap<String, ImageRequestMeta>>>().await?;
        
        match requests_map {
            Some(map) => Ok(map.into_values().collect()),
            None => Ok(vec![]),
        }
    }
    
    /// Get all outgoing image requests for a user
    pub async fn get_outgoing_image_requests(&self, user_id: &str) -> Result<Vec<ImageRequestMeta>, reqwest::Error> {
        let url = format!("{}/image_requests/{}/outgoing.json", self.base_url, user_id);
        let resp = self.client.get(&url).send().await?;
        let requests_map = resp.json::<Option<HashMap<String, ImageRequestMeta>>>().await?;
        
        match requests_map {
            Some(map) => Ok(map.into_values().collect()),
            None => Ok(vec![]),
        }
    }
    
    /// Update an image request status
    pub async fn update_image_request_status(&self, from_user_id: &str, to_user_id: &str, request_id: &str, status: &str) -> Result<(), reqwest::Error> {
        // Update in both users' lists in parallel for better performance
        let url_from = format!("{}/image_requests/{}/outgoing/{}/status.json", 
            self.base_url, from_user_id, request_id);
        let url_to = format!("{}/image_requests/{}/incoming/{}/status.json", 
            self.base_url, to_user_id, request_id);
        
        // Execute both updates in parallel
        let (result_from, result_to) = tokio::join!(
            self.client.put(&url_from).json(&status).send(),
            self.client.put(&url_to).json(&status).send()
        );
        
        result_from?;
        result_to?;
        
        Ok(())
    }
    
    /// Delete an image request from both users' lists
    pub async fn delete_image_request(&self, from_user_id: &str, to_user_id: &str, request_id: &str) -> Result<(), reqwest::Error> {
        // Delete from both users' lists in parallel
        let url_from = format!("{}/image_requests/{}/outgoing/{}.json", 
            self.base_url, from_user_id, request_id);
        let url_to = format!("{}/image_requests/{}/incoming/{}.json", 
            self.base_url, to_user_id, request_id);
        
        // Execute both deletes in parallel
        let (result_from, result_to) = tokio::join!(
            self.client.delete(&url_from).send(),
            self.client.delete(&url_to).send()
        );
        
        result_from?;
        result_to?;
        
        Ok(())
    }
}

/// Metadata for received images stored in Firebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedImageMeta {
    pub image_id: String,
    pub from_user: String,
    pub remaining_views: u8,
    pub max_views: u8,
    pub received_at: i64,
    pub encrypted_data_base64: String,  // Store the encrypted image data
}

/// Metadata for notes stored in Firebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMeta {
    pub note_id: String,
    pub from_user: String,
    pub content: String,
    pub timestamp: i64,
}

/// Metadata for image requests stored in Firebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequestMeta {
    pub request_id: String,
    pub from_user_id: String,
    pub from_username: String,
    pub to_user_id: String,
    pub to_username: String,
    pub image_index: usize,
    pub timestamp: i64,
    pub status: String,  // "pending", "accepted", "rejected"
    #[serde(default = "default_quota")]
    pub quota: u8,  // View quota (1-99), defaults to 1 for old data
}

fn default_quota() -> u8 {
    1
}