
use serde::{Deserialize, Serialize};

pub struct FireBaseClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UserStatus { Online, Offline, Idle }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub status: UserStatus,
    pub last_seen: i64,
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

    // ==================== USER GETTERS ====================

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
        let gallery = resp.json::<Option<Vec<String>>>().await?;
        Ok(gallery.unwrap_or_default())
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

    pub async fn update_last_seen(&self, id: &str, timestamp: i64) -> Result<(), reqwest::Error> {
        let url = format!("{}/users/{}/last_seen.json", self.base_url, id);
        self.client.put(&url).json(&timestamp).send().await?;
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