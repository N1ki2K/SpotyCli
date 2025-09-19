use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: Option<Album>,
    pub duration_ms: u32,
    pub popularity: u8,
    pub preview_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub genres: Option<Vec<String>>,
    pub popularity: Option<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artists: Vec<Artist>,
    pub release_date: Option<String>,
    pub total_tracks: u32,
    pub images: Option<Vec<Image>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner: User,
    pub tracks: Option<PlaylistTracks>,
    pub public: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlaylistTracks {
    pub total: u32,
    pub items: Option<Vec<PlaylistTrack>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlaylistTrack {
    pub track: Option<Track>,
    pub added_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Image {
    pub url: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    pub tracks: Option<SearchTracks>,
    pub artists: Option<SearchArtists>,
    pub albums: Option<SearchAlbums>,
    pub playlists: Option<SearchPlaylists>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchTracks {
    pub items: Vec<Track>,
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchArtists {
    pub items: Vec<Artist>,
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchAlbums {
    pub items: Vec<Album>,
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchPlaylists {
    pub items: Vec<Playlist>,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub current_view: ViewType,
    pub search_query: String,
    pub search_results: Option<SearchResponse>,
    pub selected_item: usize,
    pub current_track: Option<Track>,
    pub is_playing: bool,
    pub volume: u8,
    pub user_playlists: Vec<Playlist>,
    pub user_albums: Vec<Album>,
    pub user_artists: Vec<Artist>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    Search,
    Library,
    Playlists,
    Albums,
    Artists,
    Player,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_view: ViewType::Search,
            search_query: String::new(),
            search_results: None,
            selected_item: 0,
            current_track: None,
            is_playing: false,
            volume: 80,
            user_playlists: Vec::new(),
            user_albums: Vec::new(),
            user_artists: Vec::new(),
        }
    }
}