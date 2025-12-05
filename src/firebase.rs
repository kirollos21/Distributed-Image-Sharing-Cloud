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
    pub status: NodeStatus,
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

    pub async fn get_node_address(&self, node_id: u8) -> Result<Option<String>, reqwest::Error> {
        let url = format!("{}/nodes/node{}/address.json", self.base_url, node_id);
        let resp = self.client.get(&url).send().await?;
        let address = resp.json::<Option<String>>().await?;
        Ok(address)
    }
}