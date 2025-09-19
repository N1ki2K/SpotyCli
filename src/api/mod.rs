use anyhow::{anyhow, Result};
use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use crate::models::*;

#[derive(Debug, Clone)]
pub struct SpotifyClient {
    client: Client,
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

impl SpotifyClient {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
            access_token: None,
            base_url: "https://api.spotify.com/v1".to_string(),
        }
    }

    pub async fn authenticate(&mut self) -> Result<()> {
        let auth_string = format!("{}:{}", self.client_id, self.client_secret);
        let encoded = base64::engine::general_purpose::STANDARD.encode(auth_string.as_bytes());

        let mut params = HashMap::new();
        params.insert("grant_type", "client_credentials");

        let response = self
            .client
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", format!("Basic {}", encoded))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let token_response: TokenResponse = response.json().await?;
            self.access_token = Some(token_response.access_token);
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(anyhow!("Authentication failed: {}", error_text))
        }
    }

    async fn make_request<T>(&self, endpoint: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| anyhow!("Not authenticated"))?;

        let url = format!("{}/{}", self.base_url, endpoint);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if response.status().is_success() {
            let result = response.json().await?;
            Ok(result)
        } else {
            let error_text = response.text().await?;
            Err(anyhow!("API request failed: {}", error_text))
        }
    }

    pub async fn search(&self, query: &str, search_type: &str, limit: u32) -> Result<SearchResponse> {
        let encoded_query = urlencoding::encode(query);
        let endpoint = format!(
            "search?q={}&type={}&limit={}",
            encoded_query, search_type, limit
        );
        self.make_request(&endpoint).await
    }

    pub async fn get_track(&self, track_id: &str) -> Result<Track> {
        let endpoint = format!("tracks/{}", track_id);
        self.make_request(&endpoint).await
    }

    pub async fn get_album(&self, album_id: &str) -> Result<Album> {
        let endpoint = format!("albums/{}", album_id);
        self.make_request(&endpoint).await
    }

    pub async fn get_artist(&self, artist_id: &str) -> Result<Artist> {
        let endpoint = format!("artists/{}", artist_id);
        self.make_request(&endpoint).await
    }

    pub async fn get_playlist(&self, playlist_id: &str) -> Result<Playlist> {
        let endpoint = format!("playlists/{}", playlist_id);
        self.make_request(&endpoint).await
    }

    pub async fn get_featured_playlists(&self, limit: u32) -> Result<SearchPlaylists> {
        let endpoint = format!("browse/featured-playlists?limit={}", limit);
        let response: serde_json::Value = self.make_request(&endpoint).await?;

        if let Some(playlists) = response.get("playlists") {
            let search_playlists: SearchPlaylists = serde_json::from_value(playlists.clone())?;
            Ok(search_playlists)
        } else {
            Err(anyhow!("No playlists found in response"))
        }
    }

    pub async fn get_new_releases(&self, limit: u32) -> Result<SearchAlbums> {
        let endpoint = format!("browse/new-releases?limit={}", limit);
        let response: serde_json::Value = self.make_request(&endpoint).await?;

        if let Some(albums) = response.get("albums") {
            let search_albums: SearchAlbums = serde_json::from_value(albums.clone())?;
            Ok(search_albums)
        } else {
            Err(anyhow!("No albums found in response"))
        }
    }
}