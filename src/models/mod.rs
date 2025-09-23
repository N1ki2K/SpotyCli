use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub uri: String,
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
    pub uri: Option<String>,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CurrentPlayback {
    pub device: Device,
    pub shuffle_state: bool,
    pub repeat_state: String,
    pub timestamp: u64,
    pub context: Option<PlaybackContext>,
    pub progress_ms: Option<u64>,
    pub item: Option<Track>,
    pub currently_playing_type: String,
    pub is_playing: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Device {
    pub id: Option<String>,
    pub is_active: bool,
    pub is_private_session: bool,
    pub is_restricted: bool,
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub volume_percent: Option<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlaybackContext {
    pub external_urls: Option<ExternalUrls>,
    pub href: String,
    #[serde(rename = "type")]
    pub context_type: String,
    pub uri: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExternalUrls {
    pub spotify: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceList {
    pub devices: Vec<Device>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecentlyPlayedResponse {
    pub items: Vec<PlayHistoryItem>,
    pub next: Option<String>,
    pub cursors: Option<Cursors>,
    pub limit: u32,
    pub href: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayHistoryItem {
    pub track: Track,
    pub played_at: String,
    pub context: Option<PlaybackContext>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cursors {
    pub after: Option<String>,
    pub before: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlaylistsResponse {
    pub items: Vec<Playlist>,
    pub total: u32,
    pub limit: u32,
    pub offset: u32,
    pub href: String,
    pub next: Option<String>,
    pub previous: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueueResponse {
    pub currently_playing: Option<Track>,
    pub queue: Vec<Track>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecentlyPlayedTrack {
    pub track: Track,
    pub played_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecentlyPlayedStorage {
    pub tracks: Vec<RecentlyPlayedTrack>,
}

impl RecentlyPlayedStorage {
    const MAX_TRACKS: usize = 30;
    const STORAGE_FILE: &'static str = ".spotify_recently_played";

    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
        }
    }

    pub fn load() -> Self {
        if Path::new(Self::STORAGE_FILE).exists() {
            if let Ok(content) = fs::read_to_string(Self::STORAGE_FILE) {
                if let Ok(storage) = serde_json::from_str::<RecentlyPlayedStorage>(&content) {
                    return storage;
                }
            }
        }
        Self::new()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(Self::STORAGE_FILE, content)?;
        Ok(())
    }

    pub fn add_track(&mut self, track: Track, played_at: Option<String>) {
        let played_at = played_at.unwrap_or_else(|| {
            chrono::Utc::now().to_rfc3339()
        });

        let recent_track = RecentlyPlayedTrack {
            track,
            played_at,
        };

        // Remove if already exists (to update timestamp)
        self.tracks.retain(|t| t.track.id != recent_track.track.id);

        // Add to front
        self.tracks.insert(0, recent_track);

        // Keep only last 30 tracks
        if self.tracks.len() > Self::MAX_TRACKS {
            self.tracks.truncate(Self::MAX_TRACKS);
        }
    }

    pub fn get_tracks(&self) -> Vec<Track> {
        self.tracks.iter().map(|rt| rt.track.clone()).collect()
    }

    pub fn update_from_spotify(&mut self, spotify_recent: Vec<PlayHistoryItem>) {
        for item in spotify_recent {
            self.add_track(item.track, Some(item.played_at));
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShuffleMode {
    Off,
    On,
    SmartShuffle,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub current_view: ViewType,
    pub search_query: String,
    pub search_results: Option<SearchResponse>,
    #[allow(dead_code)]
    pub selected_item: usize,
    pub current_track: Option<Track>,
    pub is_playing: bool,
    pub current_playback: Option<CurrentPlayback>,
    pub user_authenticated: bool,
    pub auth_message: String,
    #[allow(dead_code)]
    pub volume: u8,
    pub shuffle_mode: ShuffleMode,
    pub user_playlists: Vec<Playlist>,
    pub selected_playlist: Option<Playlist>,
    pub selected_playlist_tracks: Vec<Track>,
    pub liked_songs: Vec<Track>,
    pub user_albums: Vec<Album>,
    pub user_artists: Vec<Artist>,
    pub recently_played: Vec<Track>,
    pub recently_played_storage: RecentlyPlayedStorage,
    pub queue: Vec<Track>,
    pub error_logs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    Search,
    LikedSongs,
    Playlists,
    PlaylistTracks,
    Queue,
    Albums,
    Artists,
    Errors,
    #[allow(dead_code)]
    Player,
}

impl Default for AppState {
    fn default() -> Self {
        let storage = RecentlyPlayedStorage::load();
        let recently_played = if storage.tracks.is_empty() {
            create_sample_recent_tracks()
        } else {
            storage.get_tracks()
        };

        Self {
            current_view: ViewType::Search,
            search_query: String::new(),
            search_results: None,
            selected_item: 0,
            current_track: None,
            is_playing: false,
            current_playback: None,
            user_authenticated: false,
            auth_message: String::new(),
            volume: 80,
            shuffle_mode: ShuffleMode::Off,
            user_playlists: Vec::new(),
            selected_playlist: None,
            selected_playlist_tracks: Vec::new(),
            liked_songs: Vec::new(),
            user_albums: Vec::new(),
            user_artists: Vec::new(),
            recently_played,
            recently_played_storage: storage,
            queue: Vec::new(),
            error_logs: Vec::new(),
        }
    }
}

fn create_sample_recent_tracks() -> Vec<Track> {
    vec![
        Track {
            id: "recent1".to_string(),
            name: "Nothing Else Matters".to_string(),
            uri: "spotify:track:recent1".to_string(),
            artists: vec![Artist {
                id: "metallica".to_string(),
                name: "Metallica".to_string(),
                genres: None,
                popularity: None,
            }],
            album: None,
            duration_ms: 387000,
            popularity: 95,
            preview_url: None,
        },
        Track {
            id: "recent2".to_string(),
            name: "Enter Sandman".to_string(),
            uri: "spotify:track:recent2".to_string(),
            artists: vec![Artist {
                id: "metallica".to_string(),
                name: "Metallica".to_string(),
                genres: None,
                popularity: None,
            }],
            album: None,
            duration_ms: 331000,
            popularity: 98,
            preview_url: None,
        },
        Track {
            id: "recent3".to_string(),
            name: "Master of Puppets".to_string(),
            uri: "spotify:track:recent3".to_string(),
            artists: vec![Artist {
                id: "metallica".to_string(),
                name: "Metallica".to_string(),
                genres: None,
                popularity: None,
            }],
            album: None,
            duration_ms: 515000,
            popularity: 92,
            preview_url: None,
        },
        Track {
            id: "recent4".to_string(),
            name: "One".to_string(),
            uri: "spotify:track:recent4".to_string(),
            artists: vec![Artist {
                id: "metallica".to_string(),
                name: "Metallica".to_string(),
                genres: None,
                popularity: None,
            }],
            album: None,
            duration_ms: 446000,
            popularity: 90,
            preview_url: None,
        },
        Track {
            id: "recent5".to_string(),
            name: "For Whom the Bell Tolls".to_string(),
            uri: "spotify:track:recent5".to_string(),
            artists: vec![Artist {
                id: "metallica".to_string(),
                name: "Metallica".to_string(),
                genres: None,
                popularity: None,
            }],
            album: None,
            duration_ms: 309000,
            popularity: 88,
            preview_url: None,
        },
    ]
}