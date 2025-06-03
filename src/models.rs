use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyMetadata {
    #[serde(rename = "artist_name")]
    pub artist_name: Option<String>,
    #[serde(rename = "album_title")]
    pub album_title: Option<String>,
    #[serde(rename = "title")]
    pub title: Option<String>,
    #[serde(rename = "duration")]
    pub duration: Option<String>,
    #[serde(rename = "image_url")]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTrackData {
    pub metadata: Option<SpotifyMetadata>,
    pub uri: Option<String>,
    pub uid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    pub track_title: String,
    pub artist_name: String,
    pub album_title: String,
    pub duration: String,
    pub image_url: String,
    pub uri: String,
    pub raw_data: String,
}

impl Default for PlayerData {
    fn default() -> Self {
        PlayerData {
            track_title: "No track info yet".to_string(),
            artist_name: "Unknown Artist".to_string(),
            album_title: "Unknown Album".to_string(),
            duration: "0".to_string(),
            image_url: "".to_string(),
            uri: "".to_string(),
            raw_data: "".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ClientType {
    Spotify,
    WebInterface,
    #[allow(dead_code)]
    Unknown,
}

pub type PlayerState = Arc<RwLock<PlayerData>>;
pub type ClientRegistry = Arc<RwLock<HashMap<String, ClientType>>>;
