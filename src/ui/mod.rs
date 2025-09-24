use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame, Terminal,
};
use std::io;

use crate::models::{AppState, ViewType, ShuffleMode};
use crate::api::SpotifyClient;
use crate::auth::SpotifyAuth;

pub struct App {
    pub state: AppState,
    pub list_state: ListState,
    pub input_mode: bool,
    pub spotify_client: Option<SpotifyClient>,
    pub auth_client: Option<SpotifyAuth>,
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            state: AppState::default(),
            list_state,
            input_mode: false,
            spotify_client: None,
            auth_client: None,
        }
    }

    pub fn set_spotify_client(&mut self, client: SpotifyClient) {
        self.spotify_client = Some(client);
    }

    pub fn set_auth_client(&mut self, client: SpotifyAuth) {
        self.auth_client = Some(client);
    }


    async fn trigger_search(&mut self) {
        if self.state.search_query.is_empty() {
            // Empty search - show recently played tracks
            self.state.search_results = None;
            self.state.recently_played = self.state.recently_played_storage.get_tracks();
            self.list_state.select(Some(0));
            return;
        }

        if !self.state.search_query.is_empty() {
            if let Some(ref client) = self.spotify_client {
                match client.search(&self.state.search_query, "track", 10).await {
                    Ok(search_results) => {
                        self.state.search_results = Some(search_results);
                        self.list_state.select(Some(0));
                    },
                    Err(_) => {
                        // Fallback to mock results if search fails
                        use crate::models::*;

                        let sample_tracks = vec![
                            Track {
                                id: "error1".to_string(),
                                name: format!("No results for '{}'", self.state.search_query),
                                uri: "spotify:track:error1".to_string(),
                                artists: vec![Artist {
                                    id: "error".to_string(),
                                    name: "Search Error".to_string(),
                                    genres: None,
                                    popularity: None,
                                }],
                                album: None,
                                duration_ms: 0,
                                popularity: 0,
                                preview_url: None,
                            },
                        ];

                        let search_response = SearchResponse {
                            tracks: Some(SearchTracks {
                                items: sample_tracks,
                                total: 1,
                            }),
                            artists: None,
                            albums: None,
                            playlists: None,
                        };

                        self.state.search_results = Some(search_response);
                        self.list_state.select(Some(0));
                    }
                }
            }
        }
    }

    async fn authenticate_user(&mut self) {
        if self.state.user_authenticated {
            // Check for available devices
            if let Some(ref client) = self.spotify_client {
                match client.get_available_devices().await {
                    Ok(devices) => {
                        if devices.devices.is_empty() {
                            self.state.auth_message = "❌ No Spotify devices found! Open Spotify app first.".to_string();
                        } else {
                            let active_device = devices.devices.iter().find(|d| d.is_active);
                            if let Some(device) = active_device {
                                self.state.auth_message = format!("✅ Connected to: {}", device.name);
                            } else {
                                self.state.auth_message = format!("⚠️ {} devices found but none active. Start playing something in Spotify first.", devices.devices.len());
                            }
                        }

                        // Load recently played tracks and playlists when device check succeeds
                        self.load_recently_played_from_spotify().await;
                        self.load_user_playlists().await;
                    },
                    Err(e) => {
                        self.state.auth_message = format!("❌ Device check failed: {}", e);
                    }
                }
            } else {
                self.state.auth_message = "✅ Authenticated but no client available".to_string();
            }
        } else if self.auth_client.is_some() {
            // Show authentication instructions
            self.state.auth_message = "🔐 Authentication required! Exit app (press 'q') and run: cargo run --bin authenticate".to_string();
        } else {
            self.state.auth_message = "❌ Authentication client not available".to_string();
        }
    }


    pub async fn load_recently_played_from_spotify(&mut self) {
        if self.state.user_authenticated {
            if let Some(ref client) = self.spotify_client {
                match client.get_recently_played(30).await {
                    Ok(response) => {
                        self.state.recently_played_storage.update_from_spotify(response.items);
                        self.state.recently_played = self.state.recently_played_storage.get_tracks();

                        // Save to file
                        if let Err(e) = self.state.recently_played_storage.save() {
                            self.log_error(format!("Failed to save recently played: {}", e));
                        } else {
                            self.state.auth_message = format!("✅ Loaded {} recently played tracks", self.state.recently_played.len());
                        }
                    },
                    Err(e) => {
                        self.state.auth_message = format!("⚠️ Failed to load recently played: {}", e);
                    }
                }
            }
        }
    }

    pub async fn load_user_playlists(&mut self) {
        if self.state.user_authenticated {
            if let Some(ref client) = self.spotify_client {
                self.state.auth_message = "🔄 Loading playlists...".to_string();
                match client.get_user_playlists(50, 0).await {
                    Ok(response) => {
                        self.state.user_playlists = response.items;
                        self.state.auth_message = format!("✅ Loaded {} playlists", self.state.user_playlists.len());
                    },
                    Err(e) => {
                        self.state.auth_message = format!("⚠️ Failed to load playlists: {}", e);
                    }
                }
            } else {
                self.state.auth_message = "❌ No Spotify client available".to_string();
            }
        } else {
            self.state.auth_message = "❌ Authentication required to load playlists".to_string();
        }
    }

    async fn load_queue(&mut self) {
        if self.state.user_authenticated {
            if let Some(ref client) = self.spotify_client {
                self.state.auth_message = "🔄 Loading queue...".to_string();
                match client.get_queue().await {
                    Ok(response) => {
                        self.state.queue = response.queue;
                        self.state.auth_message = format!("✅ Loaded {} tracks in queue", self.state.queue.len());
                    },
                    Err(e) => {
                        self.state.auth_message = format!("⚠️ Failed to load queue: {}", e);
                    }
                }
            } else {
                self.state.auth_message = "❌ No Spotify client available".to_string();
            }
        } else {
            self.state.auth_message = "❌ Authentication required to load queue".to_string();
        }
    }

    pub async fn load_selected_playlist_tracks(&mut self, playlist_id: &str) {
        if self.state.user_authenticated {
            if let Some(ref client) = self.spotify_client {
                self.state.auth_message = "🔄 Loading playlist tracks...".to_string();
                match client.get_playlist_tracks(playlist_id, 50, 0).await {
                    Ok(tracks_response) => {
                        if let Some(items) = tracks_response.items {
                            self.state.selected_playlist_tracks = items
                                .into_iter()
                                .filter_map(|item| item.track)
                                .collect();
                            self.state.current_view = ViewType::PlaylistTracks;
                            self.list_state.select(Some(0)); // Reset selection to first item
                            self.state.auth_message = format!("✅ Loaded {} tracks", self.state.selected_playlist_tracks.len());
                        } else {
                            self.state.selected_playlist_tracks = Vec::new();
                            self.state.current_view = ViewType::PlaylistTracks;
                            self.state.auth_message = "⚠️ Playlist has no tracks".to_string();
                        }
                    },
                    Err(e) => {
                        self.state.auth_message = format!("❌ Failed to load playlist tracks: {}", e);
                    }
                }
            } else {
                self.state.auth_message = "❌ No Spotify client available".to_string();
            }
        } else {
            self.state.auth_message = "❌ Authentication required to load playlist tracks".to_string();
        }
    }

    pub async fn load_liked_songs(&mut self) {
        if self.state.user_authenticated {
            if let Some(ref client) = self.spotify_client {
                self.state.auth_message = "🔄 Loading liked songs...".to_string();
                match client.get_liked_songs(50, 0).await {
                    Ok(response) => {
                        if let Some(items) = response.get("items").and_then(|v| v.as_array()) {
                            let mut tracks = Vec::new();
                            for item in items {
                                if let Some(track_obj) = item.get("track") {
                                    if let Ok(track) = serde_json::from_value::<crate::models::Track>(track_obj.clone()) {
                                        tracks.push(track);
                                    }
                                }
                            }
                            self.state.liked_songs = tracks;
                            self.state.auth_message = format!("✅ Loaded {} liked songs", self.state.liked_songs.len());
                        } else {
                            self.state.liked_songs = Vec::new();
                            self.state.auth_message = "⚠️ No liked songs found".to_string();
                        }
                    },
                    Err(e) => {
                        self.state.auth_message = format!("❌ Failed to load liked songs: {}", e);
                    }
                }
            } else {
                self.state.auth_message = "❌ No Spotify client available".to_string();
            }
        } else {
            self.state.auth_message = "❌ Authentication required to load liked songs".to_string();
        }
    }

    async fn open_selected_playlist(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.state.user_playlists.len() {
                let playlist = self.state.user_playlists[selected].clone();
                self.state.selected_playlist = Some(playlist.clone());
                self.load_selected_playlist_tracks(&playlist.id).await;
            }
        }
    }

    async fn play_selected_track(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            let track = match self.state.current_view {
                ViewType::Search => {
                    if let Some(ref search_results) = self.state.search_results {
                        if let Some(ref tracks) = search_results.tracks {
                            if selected < tracks.items.len() {
                                Some(tracks.items[selected].clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        // Recently played tracks
                        if selected < self.state.recently_played.len() {
                            Some(self.state.recently_played[selected].clone())
                        } else {
                            None
                        }
                    }
                }
                ViewType::PlaylistTracks => {
                    if selected < self.state.selected_playlist_tracks.len() {
                        Some(self.state.selected_playlist_tracks[selected].clone())
                    } else {
                        None
                    }
                }
                ViewType::LikedSongs => {
                    if !self.state.liked_songs.is_empty() {
                        // Use actual liked songs
                        if selected < self.state.liked_songs.len() {
                            Some(self.state.liked_songs[selected].clone())
                        } else {
                            None
                        }
                    } else {
                        // Use sample liked songs from recently played for now
                        if selected < self.state.recently_played.len() {
                            Some(self.state.recently_played[selected].clone())
                        } else {
                            None
                        }
                    }
                }
                ViewType::Queue => {
                    if selected < self.state.queue.len() {
                        Some(self.state.queue[selected].clone())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(track) = track {
                if self.state.user_authenticated {
                    if let Some(ref client) = self.spotify_client {
                        let client_clone = client.clone(); // Clone early to avoid borrowing issues
                        let play_result = match self.state.current_view {
                            ViewType::PlaylistTracks => {
                                // Play playlist with context for continuous playback
                                if let Some(ref playlist) = self.state.selected_playlist {
                                    let playlist_uri = playlist.uri.clone()
                                        .unwrap_or_else(|| format!("spotify:playlist:{}", playlist.id));
                                    client.play_playlist_with_offset(&playlist_uri, selected).await
                                } else {
                                    // Fallback to playing individual track
                                    client.play_track(&track.uri).await
                                }
                            }
                            ViewType::LikedSongs => {
                                // Play liked songs with context
                                let track_uris: Vec<String> = self.state.liked_songs.iter()
                                    .map(|t| t.uri.clone())
                                    .collect();
                                if !track_uris.is_empty() {
                                    client.play_tracks_with_offset(&track_uris, selected).await
                                } else {
                                    // Fallback to recently played
                                    let track_uris: Vec<String> = self.state.recently_played.iter()
                                        .map(|t| t.uri.clone())
                                        .collect();
                                    client.play_tracks_with_offset(&track_uris, selected).await
                                }
                            }
                            ViewType::Search | ViewType::Albums | ViewType::Artists | ViewType::Queue => {
                                // For individual tracks from search/albums/artists/queue, start radio to continue with similar songs
                                match client.start_radio_from_track(&track.uri).await {
                                    Ok(logs) => {
                                        // Add all radio logs to the error logs tab
                                        for log in logs {
                                            self.log_radio(log);
                                        }
                                        Ok(())
                                    }
                                    Err(e) => Err(e)
                                }
                            }
                            _ => {
                                // For other views like recently played, also start radio
                                match client.start_radio_from_track(&track.uri).await {
                                    Ok(logs) => {
                                        // Add all radio logs to the error logs tab
                                        for log in logs {
                                            self.log_radio(log);
                                        }
                                        Ok(())
                                    }
                                    Err(e) => Err(e)
                                }
                            }
                        };

                        match play_result {
                            Ok(_) => {
                                // Clear the current queue when starting a new song
                                self.state.queue.clear();
                                self.log_radio("🔄 Queue cleared - starting fresh".to_string());

                                // Load the new queue after a short delay to allow Spotify to populate it
                                tokio::spawn({
                                    let client_for_spawn = client_clone.clone();
                                    async move {
                                        // Wait a moment for Spotify to populate the queue with new tracks
                                        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                                        let _ = client_for_spawn.get_queue().await;
                                    }
                                });

                                let message = match self.state.current_view {
                                    ViewType::Search | ViewType::Albums | ViewType::Artists | ViewType::Queue => {
                                        format!("📻 Starting radio: {} (Building playlist with similar tracks...)", track.name)
                                    }
                                    ViewType::PlaylistTracks => {
                                        format!("▶ Playing from playlist: {}", track.name)
                                    }
                                    ViewType::LikedSongs => {
                                        format!("❤️ Playing from liked songs: {}", track.name)
                                    }
                                    _ => {
                                        format!("📻 Starting radio: {} (Building playlist with similar tracks...)", track.name)
                                    }
                                };
                                self.state.auth_message = message;
                                self.state.current_track = Some(track.clone());
                                self.state.is_playing = true;

                                // Add to recently played storage
                                self.state.recently_played_storage.add_track(track, None);
                                let _ = self.state.recently_played_storage.save();

                                // Sync with Spotify after a short delay
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                self.sync_playback_state().await;
                            }
                            Err(e) => {
                                self.log_error(format!("❌ PLAY ERROR: {}", e));
                                let error_msg = e.to_string();
                                if error_msg.contains("NO_ACTIVE_DEVICE") {
                                    self.state.auth_message = "❌ No active device! Open Spotify app first.".to_string();
                                } else if error_msg.contains("PREMIUM_REQUIRED") {
                                    self.state.auth_message = "❌ Spotify Premium required for playback.".to_string();
                                } else {
                                    self.state.auth_message = format!("❌ Play error: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    self.state.auth_message = "❌ Authentication required for playback".to_string();
                }
            }
        }
    }

    async fn toggle_playback(&mut self) {
        if self.state.user_authenticated {
            if let Some(ref client) = self.spotify_client {
                let result = if self.state.is_playing {
                    client.pause_playback().await
                } else {
                    client.resume_playback().await
                };

                match result {
                    Ok(_) => {
                        self.state.is_playing = !self.state.is_playing;
                        self.state.auth_message = format!("🎵 {}", if self.state.is_playing { "Resumed" } else { "Paused" });

                        // Sync with Spotify after a short delay
                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                        self.sync_playback_state().await;
                    },
                    Err(e) => {
                        self.state.auth_message = format!("❌ Playback error: {}", e);
                    }
                }
            }
        }
    }

    async fn next_track(&mut self) {
        if self.state.user_authenticated {
            if let Some(ref client) = self.spotify_client {
                match client.next_track().await {
                    Ok(_) => {
                        self.state.auth_message = "⏭ Next track".to_string();

                        // Sync with Spotify after a delay to allow track change
                        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                        self.sync_playback_state().await;
                    },
                    Err(e) => {
                        self.state.auth_message = format!("❌ Next track error: {}", e);
                    }
                }
            }
        } else {
            self.state.auth_message = "❌ Authentication required".to_string();
        }
    }

    async fn previous_track(&mut self) {
        if self.state.user_authenticated {
            if let Some(ref client) = self.spotify_client {
                match client.previous_track().await {
                    Ok(_) => {
                        self.state.auth_message = "⏮ Previous track".to_string();

                        // Sync with Spotify after a delay to allow track change
                        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                        self.sync_playback_state().await;
                    },
                    Err(e) => {
                        self.state.auth_message = format!("❌ Previous track error: {}", e);
                    }
                }
            }
        } else {
            self.state.auth_message = "❌ Authentication required".to_string();
        }
    }

    async fn toggle_shuffle(&mut self) {
        if self.state.user_authenticated {
            if let Some(ref client) = self.spotify_client {
                // Cycle through: Off -> On -> SmartShuffle -> Off
                let (new_mode, result) = match self.state.shuffle_mode {
                    ShuffleMode::Off => {
                        (ShuffleMode::On, client.set_shuffle(true).await)
                    }
                    ShuffleMode::On => {
                        (ShuffleMode::SmartShuffle, client.set_smart_shuffle(true).await)
                    }
                    ShuffleMode::SmartShuffle => {
                        (ShuffleMode::Off, client.set_shuffle(false).await)
                    }
                };

                match result {
                    Ok(_) => {
                        self.state.shuffle_mode = new_mode.clone();
                        let mode_text = match new_mode {
                            ShuffleMode::Off => "🔀 Shuffle: Off",
                            ShuffleMode::On => "🔀 Shuffle: On",
                            ShuffleMode::SmartShuffle => "🔀 Smart Shuffle: On",
                        };
                        self.state.auth_message = mode_text.to_string();

                        // Sync after a short delay
                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                        self.sync_playback_state().await;
                    },
                    Err(e) => {
                        self.log_error(format!("❌ SHUFFLE ERROR: {}", e));
                        let error_msg = e.to_string();
                        if error_msg.contains("NO_ACTIVE_DEVICE") {
                            self.state.auth_message = "❌ No active device! Open Spotify app first.".to_string();
                        } else if error_msg.contains("PREMIUM_REQUIRED") {
                            self.state.auth_message = "❌ Spotify Premium required for shuffle control.".to_string();
                        } else {
                            self.state.auth_message = format!("❌ Shuffle error: {}", e);
                        }
                    }
                }
            }
        } else {
            self.state.auth_message = "❌ Authentication required for shuffle control".to_string();
        }
    }

    async fn toggle_like_selected_track(&mut self) {
        let user_authenticated = self.state.user_authenticated;
        let current_view = self.state.current_view.clone();
        let selected = self.list_state.selected();

        if let Some(selected) = selected {
            let track = match current_view {
                ViewType::Search => {
                    if let Some(ref search_results) = self.state.search_results {
                        if let Some(ref tracks) = search_results.tracks {
                            if selected < tracks.items.len() {
                                Some(tracks.items[selected].clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        if selected < self.state.recently_played.len() {
                            Some(self.state.recently_played[selected].clone())
                        } else {
                            None
                        }
                    }
                }
                ViewType::PlaylistTracks => {
                    if selected < self.state.selected_playlist_tracks.len() {
                        Some(self.state.selected_playlist_tracks[selected].clone())
                    } else {
                        None
                    }
                }
                ViewType::LikedSongs => {
                    if !self.state.liked_songs.is_empty() {
                        if selected < self.state.liked_songs.len() {
                            Some(self.state.liked_songs[selected].clone())
                        } else {
                            None
                        }
                    } else {
                        if selected < self.state.recently_played.len() {
                            Some(self.state.recently_played[selected].clone())
                        } else {
                            None
                        }
                    }
                }
                ViewType::Queue => {
                    if selected < self.state.queue.len() {
                        Some(self.state.queue[selected].clone())
                    } else {
                        None
                    }
                }
                _ => {
                    if selected < self.state.recently_played.len() {
                        Some(self.state.recently_played[selected].clone())
                    } else {
                        None
                    }
                }
            };

            if let Some(track) = track {
                if user_authenticated {
                    if let Some(client) = self.spotify_client.clone() {
                        // First check if the track is already liked
                        match client.check_if_liked(&track.id).await {
                            Ok(is_liked) => {
                                let result = if is_liked {
                                    client.unlike_song(&track.id).await
                                } else {
                                    client.like_song(&track.id).await
                                };

                                match result {
                                    Ok(_) => {
                                        let action = if is_liked { "💔 Removed from" } else { "❤️ Added to" };
                                        self.state.auth_message = format!("{} liked songs: {}", action, track.name);

                                        // If we're in liked songs view and we just unliked, refresh the list
                                        if is_liked && current_view == ViewType::LikedSongs {
                                            self.load_liked_songs().await;
                                        }
                                    },
                                    Err(e) => {
                                        self.log_error(format!("❌ LIKE ERROR: {}", e));
                                        let error_msg = e.to_string();
                                        if error_msg.contains("PREMIUM_REQUIRED") {
                                            self.state.auth_message = "❌ Spotify Premium required for liking songs.".to_string();
                                        } else {
                                            self.state.auth_message = format!("❌ Like error: {}", e);
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                self.log_error(format!("❌ LIKE CHECK ERROR: {}", e));
                                self.state.auth_message = format!("❌ Error checking like status: {}", e);
                            }
                        }
                    }
                } else {
                    self.state.auth_message = "❌ Authentication required for liking songs".to_string();
                }
            } else {
                self.state.auth_message = "❌ No track selected".to_string();
            }
        } else {
            self.state.auth_message = "❌ No track selected".to_string();
        }
    }

    fn log_error(&mut self, message: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        let log_entry = format!("[{}] {}", timestamp, message);
        self.state.error_logs.push(log_entry);

        // Keep only last 100 log entries
        if self.state.error_logs.len() > 100 {
            self.state.error_logs.remove(0);
        }
    }

    fn log_radio(&mut self, message: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        let log_entry = format!("[{}] RADIO: {}", timestamp, message);
        self.state.error_logs.push(log_entry);

        // Keep only last 100 log entries
        if self.state.error_logs.len() > 100 {
            self.state.error_logs.remove(0);
        }
    }

    async fn add_selected_to_queue(&mut self) {
        let user_authenticated = self.state.user_authenticated;
        let current_view = self.state.current_view.clone();
        let selected = self.list_state.selected();

        if let Some(selected) = selected {
            let track = match current_view {
                ViewType::Search => {
                    if let Some(ref search_results) = self.state.search_results {
                        if let Some(ref tracks) = search_results.tracks {
                            if selected < tracks.items.len() {
                                Some(tracks.items[selected].clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        if selected < self.state.recently_played.len() {
                            Some(self.state.recently_played[selected].clone())
                        } else {
                            None
                        }
                    }
                }
                ViewType::PlaylistTracks => {
                    if selected < self.state.selected_playlist_tracks.len() {
                        Some(self.state.selected_playlist_tracks[selected].clone())
                    } else {
                        None
                    }
                }
                ViewType::LikedSongs => {
                    if !self.state.liked_songs.is_empty() {
                        // Use actual liked songs
                        if selected < self.state.liked_songs.len() {
                            Some(self.state.liked_songs[selected].clone())
                        } else {
                            None
                        }
                    } else {
                        // Use sample liked songs from recently played for now
                        if selected < self.state.recently_played.len() {
                            Some(self.state.recently_played[selected].clone())
                        } else {
                            None
                        }
                    }
                }
                _ => {
                    if selected < self.state.recently_played.len() {
                        Some(self.state.recently_played[selected].clone())
                    } else {
                        None
                    }
                }
            };

            if let Some(track) = track {
                if user_authenticated {
                    if let Some(client) = self.spotify_client.clone() {
                        match client.add_to_queue(&track.uri).await {
                            Ok(_) => {
                                self.state.auth_message = format!("🚀 Added to queue (high priority): {}", track.name);
                                self.log_radio(format!("🚀 HIGH PRIORITY: {} added to queue", track.name));

                                // Refresh the queue after a short delay to get the updated queue
                                // and move manually added tracks to higher priority
                                tokio::spawn({
                                    let client_clone = client.clone();
                                    async move {
                                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                        let _ = client_clone.get_queue().await;
                                    }
                                });
                            },
                            Err(e) => {
                                self.log_error(format!("❌ QUEUE ERROR: {}", e));
                                let error_msg = e.to_string();
                                if error_msg.contains("NO_ACTIVE_DEVICE") {
                                    self.state.auth_message = "❌ No active device! Open Spotify app first.".to_string();
                                } else if error_msg.contains("PREMIUM_REQUIRED") {
                                    self.state.auth_message = "❌ Spotify Premium required for queue control.".to_string();
                                } else {
                                    self.state.auth_message = format!("❌ Queue error: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    self.state.auth_message = "❌ Authentication required for queue control".to_string();
                }
            }
        }
    }

    async fn volume_up(&mut self) {
        self.adjust_volume(10).await;
    }

    async fn volume_down(&mut self) {
        self.adjust_volume(-10).await;
    }

    async fn adjust_volume(&mut self, delta: i8) {
        self.log_error(format!("Volume adjust called: delta={}, user_auth={}", delta, self.state.user_authenticated));

        if self.state.user_authenticated {
            if let Some(client) = self.spotify_client.clone() {
                // First, try to get current volume from Spotify
                let current_volume = if let Ok(Some(playback)) = client.get_current_playback().await {
                    if let Some(volume) = playback.device.volume_percent {
                        self.log_error(format!("Got current volume from device: {}%", volume));
                        volume
                    } else {
                        self.log_error("Device has no volume info, using stored volume".to_string());
                        self.state.volume // fallback to stored volume
                    }
                } else {
                    self.log_error("Failed to get playback info, using stored volume".to_string());
                    self.state.volume // fallback to stored volume
                };

                let new_volume = (current_volume as i16 + delta as i16).clamp(0, 100) as u8;
                self.log_error(format!("Volume change: {} -> {} (delta: {})", current_volume, new_volume, delta));

                match client.set_volume(new_volume).await {
                    Ok(_) => {
                        self.log_error(format!("✅ Volume API call successful: set to {}%", new_volume));
                        self.state.volume = new_volume;
                        self.state.auth_message = format!("🔊 Volume: {}% ({}{})",
                            new_volume,
                            if delta > 0 { "+" } else { "" },
                            delta
                        );

                        // Sync after volume change to update display
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        self.sync_playback_state().await;
                    },
                    Err(e) => {
                        self.log_error(format!("❌ Volume API call failed: {}", e));
                        let error_msg = e.to_string();
                        if error_msg.contains("NO_ACTIVE_DEVICE") {
                            self.state.auth_message = "❌ No active device! Open Spotify app first.".to_string();
                        } else if error_msg.contains("PREMIUM_REQUIRED") {
                            self.state.auth_message = "❌ Spotify Premium required for volume control.".to_string();
                        } else {
                            self.state.auth_message = format!("❌ Volume error: {}", e);
                        }
                    }
                }
            } else {
                self.state.auth_message = "❌ No Spotify client available".to_string();
            }
        } else {
            self.state.auth_message = "❌ User authentication required for volume control".to_string();
        }
    }

    pub async fn sync_playback_state(&mut self) {
        if self.state.user_authenticated {
            if let Some(ref client) = self.spotify_client {
                match client.get_current_playback().await {
                    Ok(Some(playback)) => {
                        self.state.current_playback = Some(playback.clone());
                        self.state.is_playing = playback.is_playing;

                        // Debug info about progress data
                        let progress_info = if let Some(progress_ms) = playback.progress_ms {
                            format!(" [✅Progress: {}ms]", progress_ms)
                        } else {
                            " [❌No Progress Data]".to_string()
                        };

                        if let Some(track) = playback.item {
                            self.state.current_track = Some(track.clone());
                            if playback.is_playing {
                                self.state.auth_message = format!("✅ Playing: {}{}", track.name, progress_info);
                            } else {
                                self.state.auth_message = format!("✅ Paused: {}{}", track.name, progress_info);
                            }
                        } else {
                            self.state.current_track = None;
                            self.state.auth_message = if playback.is_playing {
                                format!("✅ SYNC SUCCESS: ▶ Playing...{}", progress_info)
                            } else {
                                format!("✅ SYNC SUCCESS: ⏸️ Paused{}", progress_info)
                            };
                        }
                    }
                    Ok(None) => {
                        // No active playback
                        self.state.current_playback = None;
                        self.state.is_playing = false;
                        self.state.current_track = None;
                        self.state.auth_message = "⏹️ No active playback - start playing on Spotify first".to_string();
                    }
                    Err(e) => {
                        self.log_error(format!("❌ SYNC ERROR: {}", e));
                        self.state.auth_message = format!("❌ Sync failed: {}", e);
                    }
                }
            }
        }
    }

    pub async fn run<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        use std::time::{Duration, Instant};
        let mut last_sync = Instant::now();
        let sync_interval = Duration::from_secs(3); // Sync every 3 seconds

        loop {
            terminal.draw(|f| self.ui(f))?;

            // Auto-sync every 3 seconds if playing
            if self.state.is_playing && self.state.user_authenticated && last_sync.elapsed() >= sync_interval {
                self.sync_playback_state().await;
                last_sync = Instant::now();
            }

            // Poll for events with timeout to allow periodic syncing
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('/') => {
                            self.input_mode = true;
                            self.state.current_view = ViewType::Search;
                        }
                        KeyCode::Char('d') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            // Prevent accidental Ctrl+D termination - show warning instead
                            self.state.auth_message = "⚠️ Use 'q' to quit or '/' to search (not Ctrl+D)".to_string();
                        }
                        KeyCode::Left if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            // Switch to previous tab
                            self.switch_tab(-1).await;
                        }
                        KeyCode::Right if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            // Switch to next tab
                            self.switch_tab(1).await;
                        }
                        KeyCode::Char('r') if key.modifiers.contains(crossterm::event::KeyModifiers::ALT) => {
                            // Alt+R: Previous track
                            self.previous_track().await;
                        }
                        KeyCode::Char('t') if key.modifiers.contains(crossterm::event::KeyModifiers::ALT) => {
                            // Alt+T: Next track
                            self.next_track().await;
                        }
                        KeyCode::Up => {
                            if !self.input_mode {
                                if let Some(selected) = self.list_state.selected() {
                                    if selected > 0 {
                                        self.list_state.select(Some(selected - 1));
                                    }
                                }
                            }
                        }
                        KeyCode::Down => {
                            if !self.input_mode {
                                let item_count = match self.state.current_view {
                                    ViewType::Search => {
                                        if let Some(ref search_results) = self.state.search_results {
                                            if let Some(ref tracks) = search_results.tracks {
                                                tracks.items.len()
                                            } else {
                                                self.state.recently_played.len()
                                            }
                                        } else {
                                            self.state.recently_played.len()
                                        }
                                    }
                                    ViewType::LikedSongs => {
                                        if !self.state.liked_songs.is_empty() {
                                            self.state.liked_songs.len()
                                        } else {
                                            9 // Number of sample items shown
                                        }
                                    }
                                    ViewType::Playlists => self.state.user_playlists.len().max(10), // Sample playlists
                                    ViewType::PlaylistTracks => self.state.selected_playlist_tracks.len(),
                                    ViewType::Queue => self.state.queue.len().max(2), // At least show "No tracks" message
                                    ViewType::Albums => self.state.user_albums.len(),
                                    ViewType::Artists => self.state.user_artists.len(),
                                    ViewType::Errors => self.state.error_logs.len(),
                                    _ => 0,
                                };

                                if let Some(selected) = self.list_state.selected() {
                                    if selected + 1 < item_count {
                                        self.list_state.select(Some(selected + 1));
                                    }
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if self.input_mode {
                                self.input_mode = false;
                                self.trigger_search().await;
                            } else {
                                match self.state.current_view {
                                    ViewType::Playlists => {
                                        self.open_selected_playlist().await;
                                    }
                                    _ => {
                                        self.play_selected_track().await;
                                    }
                                }
                            }
                        }
                        KeyCode::Esc => {
                            if self.input_mode {
                                self.input_mode = false;
                            } else {
                                match self.state.current_view {
                                    ViewType::PlaylistTracks => {
                                        // Go back to playlists view
                                        self.state.current_view = ViewType::Playlists;
                                        self.state.selected_playlist = None;
                                        self.state.selected_playlist_tracks.clear();
                                        self.list_state.select(Some(0)); // Reset selection
                                        self.state.auth_message.clear();
                                    }
                                    _ => {
                                        // Clear search results to show recently played
                                        self.state.search_results = None;
                                        self.state.search_query.clear();
                                        self.list_state.select(Some(0));
                                        self.state.auth_message.clear();
                                    }
                                }

                                // Load fresh recently played tracks from storage and Spotify
                                self.state.recently_played = self.state.recently_played_storage.get_tracks();
                                if self.state.user_authenticated {
                                    tokio::spawn(async move {
                                        // Note: Can't directly call self method in spawn,
                                        // but this will trigger a refresh when user presses 'r'
                                    });
                                }
                            }
                        }
                        KeyCode::Char(c) => {
                            if self.input_mode {
                                self.state.search_query.push(c);
                            } else {
                                match c {
                                    '1' => {
                                        self.state.current_view = ViewType::Search;
                                        self.state.auth_message.clear();
                                        self.list_state.select(Some(0));
                                    }
                                    '2' => {
                                        self.state.current_view = ViewType::LikedSongs;
                                        self.state.auth_message.clear();
                                        self.list_state.select(Some(0));
                                    }
                                    '3' => {
                                        self.state.current_view = ViewType::Playlists;
                                        self.state.auth_message.clear();
                                        self.list_state.select(Some(0));
                                    }
                                    '4' => {
                                        self.state.current_view = ViewType::Queue;
                                        self.state.auth_message.clear();
                                        self.list_state.select(Some(0));
                                        // Auto-load queue when switching to queue view
                                        self.load_queue().await;
                                    }
                                    '5' => {
                                        self.state.current_view = ViewType::Albums;
                                        self.state.auth_message.clear();
                                        self.list_state.select(Some(0));
                                    }
                                    '6' => {
                                        self.state.current_view = ViewType::Artists;
                                        self.state.auth_message.clear();
                                        self.list_state.select(Some(0));
                                    }
                                    '7' => {
                                        self.state.current_view = ViewType::Errors;
                                        self.state.auth_message.clear();
                                        self.list_state.select(Some(0));
                                    }
                                    ' ' => {
                                        self.toggle_playback().await;
                                    }
                                    'u' | 'U' => {
                                        self.authenticate_user().await;
                                    }
                                    'r' | 'R' => {
                                        self.load_recently_played_from_spotify().await;
                                    }
                                    'L' | 'l' => {
                                        self.load_liked_songs().await;
                                    }
                                    'Q' | 'q' => {
                                        self.load_queue().await;
                                    }
                                    'n' | 'N' => {
                                        self.next_track().await;
                                    }
                                    'b' | 'B' => {
                                        self.previous_track().await;
                                    }
                                    'P' | 'p' => {
                                        self.toggle_shuffle().await;
                                    }
                                    '+' | '=' => {
                                        self.state.auth_message = "🔊 Volume Up pressed...".to_string();
                                        self.volume_up().await;
                                    }
                                    '-' | '_' => {
                                        self.state.auth_message = "🔉 Volume Down pressed...".to_string();
                                        self.volume_down().await;
                                    }
                                    'm' | 'M' => {
                                        self.log_error("🎵 'm' key pressed - adding selected track to queue".to_string());
                                        self.add_selected_to_queue().await;
                                    }
                                    's' | 'S' => {
                                        self.state.auth_message = "🔄 Syncing with Spotify...".to_string();
                                        self.log_error("🔄 's' key pressed - syncing playback state".to_string());
                                        self.sync_playback_state().await;
                                    }
                                    ')' => {
                                        self.toggle_like_selected_track().await;
                                    }
                                    _ => {
                                        self.state.auth_message.clear();
                                    }
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            if self.input_mode {
                                self.state.search_query.pop();
                            }
                        }
                        _ => {}
                    }
                }
            }
            }
        }
    }

    async fn switch_tab(&mut self, direction: i32) {
        let tabs = [
            ViewType::Search,
            ViewType::LikedSongs,
            ViewType::Playlists,
            ViewType::Queue,
            ViewType::Albums,
            ViewType::Artists,
            ViewType::Errors,
        ];

        let current_index = tabs.iter().position(|tab| *tab == self.state.current_view).unwrap_or(0);
        let new_index = if direction > 0 {
            (current_index + 1) % tabs.len()
        } else {
            if current_index == 0 { tabs.len() - 1 } else { current_index - 1 }
        };

        self.state.current_view = tabs[new_index].clone();
        self.state.auth_message.clear();
        self.list_state.select(Some(0));

        // Auto-load content for specific views when switching to them
        match self.state.current_view {
            ViewType::Queue => {
                // Auto-load queue when switching to queue view
                self.load_queue().await;
            }
            _ => {}
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(f.area());

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(chunks[0]);

        self.render_sidebar(f, main_chunks[0]);
        self.render_main_content(f, main_chunks[1]);
        self.render_player(f, chunks[1]);
    }

    fn render_sidebar(&self, f: &mut Frame, area: Rect) {
        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Min(0),
            ])
            .split(area);

        // Navigation section
        let library_items = vec![
            ListItem::new("1. Search"),
            ListItem::new("2. Liked Songs"),
            ListItem::new("3. Playlists"),
            ListItem::new("4. Queue"),
            ListItem::new("5. Albums"),
            ListItem::new("6. Artists"),
            ListItem::new("7. Errors/Logs"),
        ];

        let library_list = List::new(library_items)
            .block(Block::default().title("Navigation").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        f.render_widget(library_list, sidebar_chunks[0]);

        // Playlists section
        let playlist_items: Vec<ListItem> = if self.state.user_playlists.is_empty() {
            vec![
                ListItem::new("Metallica Greatest Hits"),
                ListItem::new("Metallica: Essentials"),
                ListItem::new("Metallica: Live"),
                ListItem::new("Metallica: Complete"),
                ListItem::new("METALLICA live"),
                ListItem::new("Metallica: Studio Albums"),
                ListItem::new("Metallica: Live Sao Paulo '99"),
                ListItem::new("Metallica - Black Album"),
                ListItem::new("Metallica - Whisky in the Jar"),
                ListItem::new("METALLICA Pallavicini"),
                ListItem::new("Metallica 2002 Soliloquy 2019"),
                ListItem::new("Metallica Family Playlist"),
                ListItem::new("Metallica Load / Reload (Good Ones)"),
                ListItem::new("Metallica Chile 15 Abril 2020 - Estadio Nacional"),
            ]
        } else {
            self.state.user_playlists
                .iter()
                .map(|p| ListItem::new(p.name.clone()))
                .collect()
        };

        let playlists_list = List::new(playlist_items)
            .block(Block::default().title("Playlists").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        f.render_widget(playlists_list, sidebar_chunks[2]);
    }

    fn render_main_content(&mut self, f: &mut Frame, area: Rect) {
        match self.state.current_view {
            ViewType::Search => self.render_search(f, area),
            ViewType::LikedSongs => self.render_library(f, area),
            ViewType::Playlists => self.render_playlists(f, area),
            ViewType::PlaylistTracks => self.render_playlist_tracks(f, area),
            ViewType::Queue => self.render_queue(f, area),
            ViewType::Albums => self.render_albums(f, area),
            ViewType::Artists => self.render_artists(f, area),
            ViewType::Errors => self.render_errors(f, area),
            ViewType::Player => self.render_player_detail(f, area),
        }
    }

    fn render_search(&mut self, f: &mut Frame, area: Rect) {
        let search_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Split the main area into tracks and preview
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(search_chunks[1]);

        // Search input
        let search_style = if self.input_mode {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let search_input = Paragraph::new(self.state.search_query.as_str())
            .style(search_style)
            .block(Block::default().borders(Borders::ALL).title("Search"));

        f.render_widget(search_input, search_chunks[0]);

        // Show tracks - either search results or recently played
        if let Some(ref results) = self.state.search_results {
            // Show search results
            if let Some(ref tracks) = results.tracks {
                let track_items: Vec<ListItem> = tracks
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, track)| {
                        let artist_names: String = track
                            .artists
                            .iter()
                            .map(|a| a.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ");

                        let item_text = format!("{}. {} - {}", i + 1, track.name, artist_names);
                        ListItem::new(item_text)
                    })
                    .collect();

                let tracks_list = List::new(track_items)
                    .block(Block::default().title("🔍 Search Results (↑↓ to navigate, Enter to play)").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

                f.render_stateful_widget(tracks_list, content_chunks[0], &mut self.list_state);
            }
        } else if !self.state.search_query.is_empty() && self.input_mode {
            // Show "type to search" when in input mode
            let searching_text = Paragraph::new("🔍 Type your search and press Enter...")
                .block(Block::default().title("Search (press '/' to search)").borders(Borders::ALL));
            f.render_widget(searching_text, content_chunks[0]);
        } else {
            // Show recently played tracks when not searching
            let recent_items: Vec<ListItem> = self.state.recently_played
                .iter()
                .enumerate()
                .map(|(i, track)| {
                    let artist_names: String = track
                        .artists
                        .iter()
                        .map(|a| a.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ");

                    let item_text = format!("{}. {} - {}", i + 1, track.name, artist_names);
                    ListItem::new(item_text)
                })
                .collect();

            let tracks_list = List::new(recent_items)
                .block(Block::default().title("🎵 Recently Played (↑↓ to navigate, Enter to play)").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

            f.render_stateful_widget(tracks_list, content_chunks[0], &mut self.list_state);
        }

        // Render preview panel
        self.render_track_preview(f, content_chunks[1]);
    }

    fn render_track_preview(&self, f: &mut Frame, area: Rect) {
        let preview_text = if let Some(selected) = self.list_state.selected() {
            // Get the selected track
            let track = if let Some(ref results) = self.state.search_results {
                if let Some(ref tracks) = results.tracks {
                    tracks.items.get(selected)
                } else {
                    None
                }
            } else {
                self.state.recently_played.get(selected)
            };

            if let Some(track) = track {
                let artist_names = track
                    .artists
                    .iter()
                    .map(|a| a.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                let album_name = track
                    .album
                    .as_ref()
                    .map(|a| a.name.clone())
                    .unwrap_or("Unknown Album".to_string());

                let duration_seconds = track.duration_ms / 1000;
                let duration_formatted = format!("{}:{:02}", duration_seconds / 60, duration_seconds % 60);

                let mut preview_info = format!(
                    "🎵 {}\n\n👤 Artist(s):\n{}\n\n💿 Album:\n{}\n\n⏱️ Duration:\n{}\n\n🎚️ Popularity:\n{}/100\n\n🆔 Track ID:\n{}",
                    track.name,
                    artist_names,
                    album_name,
                    duration_formatted,
                    track.popularity,
                    track.id
                );

                // Always add a progress section at the very bottom
                preview_info.push_str("\n\n\n═══ PLAYBACK STATUS ═══");

                // Check if this track is currently playing and add progress info
                if let Some(ref current_track) = self.state.current_track {
                    if current_track.id == track.id {
                        preview_info.push_str("\n🎵 CURRENTLY PLAYING 🎵");

                        if let Some(ref playback) = self.state.current_playback {
                            let status_icon = if playback.is_playing { "▶" } else { "⏸️" };
                            preview_info.push_str(&format!("\n{} Status: {}", status_icon,
                                if playback.is_playing { "Playing" } else { "Paused" }));

                            // Add progress info
                            if let Some(progress_ms) = playback.progress_ms {
                                let progress_sec = progress_ms / 1000;
                                let progress_min = progress_sec / 60;
                                let progress_sec_remainder = progress_sec % 60;
                                let duration_min = duration_seconds / 60;
                                let duration_sec_remainder = duration_seconds % 60;

                                preview_info.push_str(&format!("\n⏱️ Progress: {}:{:02} / {}:{:02}",
                                    progress_min, progress_sec_remainder,
                                    duration_min, duration_sec_remainder));

                                // Add progress bar
                                let progress_percentage = (progress_ms as f64 / track.duration_ms as f64 * 100.0) as u8;
                                let bar_width = 20; // Bigger bar for better visibility
                                let filled = (progress_percentage as f64 / 100.0 * bar_width as f64) as usize;
                                let bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);
                                preview_info.push_str(&format!("\n[{}] {}%", bar, progress_percentage));
                            } else {
                                preview_info.push_str("\n⏱️ Progress: Unavailable");
                                let bar = "░".repeat(20);
                                preview_info.push_str(&format!("\n[{}] No data", bar));
                            }

                            preview_info.push_str(&format!("\n🎧 Device: {}", playback.device.name));
                            preview_info.push_str("\n═════════════════════");
                        } else {
                            preview_info.push_str("\n❌ No playback data");
                            preview_info.push_str("\n═════════════════════");
                        }
                    } else {
                        preview_info.push_str("\n⏹️ Not currently playing");
                        preview_info.push_str("\n💡 Press 's' to sync, then");
                        preview_info.push_str("\n   select the playing track");
                        preview_info.push_str("\n═════════════════════");
                    }
                } else {
                    preview_info.push_str("\n⏹️ No active playback");
                    preview_info.push_str("\n💡 Start music on Spotify");
                    preview_info.push_str("\n   then press 's' to sync");
                    preview_info.push_str("\n═════════════════════");
                }

                preview_info
            } else {
                "No track selected".to_string()
            }
        } else {
            "Select a track to see preview".to_string()
        };

        let preview_widget = Paragraph::new(preview_text)
            .block(Block::default().borders(Borders::ALL).title("🔍 Track Preview"))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(preview_widget, area);
    }

    fn render_library(&mut self, f: &mut Frame, area: Rect) {
        let library_items: Vec<ListItem> = if self.state.liked_songs.is_empty() {
            vec![
                ListItem::new("No liked songs loaded"),
                ListItem::new("Press 'L' to load your liked songs"),
                ListItem::new(""),
                ListItem::new("Sample Liked Songs:"),
                ListItem::new("♥ Bohemian Rhapsody - Queen"),
                ListItem::new("♥ Hotel California - Eagles"),
                ListItem::new("♥ Stairway to Heaven - Led Zeppelin"),
                ListItem::new("♥ Sweet Child O' Mine - Guns N' Roses"),
                ListItem::new("♥ Imagine - John Lennon"),
            ]
        } else {
            self.state.liked_songs
                .iter()
                .enumerate()
                .map(|(i, track)| {
                    let artists = track.artists.iter()
                        .map(|a| a.name.clone())
                        .collect::<Vec<String>>()
                        .join(", ");
                    ListItem::new(format!("{}. ♥ {} - {}", i + 1, track.name, artists))
                })
                .collect()
        };

        let library_list = List::new(library_items)
            .block(Block::default().title("🎵 Liked Songs (↑↓ to navigate, Enter to play, L to load)").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
        f.render_stateful_widget(library_list, area, &mut self.list_state);
    }

    fn render_playlists(&mut self, f: &mut Frame, area: Rect) {
        let playlist_items: Vec<ListItem> = if self.state.user_playlists.is_empty() {
            vec![
                ListItem::new("Metallica - Metallica"),
                ListItem::new("Master Of Puppets (Remastered) - Metallica"),
                ListItem::new("Metallica Through The Never (Music From The Motion Picture"),
                ListItem::new("...And Justice For All - Metallica"),
                ListItem::new("Ride The Lightning (Remastered) - Metallica"),
                ListItem::new("Kill 'Em All (Remastered) - Metallica"),
                ListItem::new("Hardwired...To Self-Destruct - Metallica"),
                ListItem::new("Death Magnetic - Metallica"),
                ListItem::new("Load - Metallica"),
                ListItem::new("Hardwired...To Self-Destruct (Deluxe) - Metallica"),
            ]
        } else {
            self.state.user_playlists
                .iter()
                .map(|p| ListItem::new(p.name.clone()))
                .collect()
        };

        let playlists_list = List::new(playlist_items)
            .block(Block::default().title("🎵 Playlists (↑↓ to navigate, Enter to open)").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        f.render_stateful_widget(playlists_list, area, &mut self.list_state);
    }

    fn render_playlist_tracks(&mut self, f: &mut Frame, area: Rect) {
        let title = if let Some(ref playlist) = self.state.selected_playlist {
            format!("🎵 {} (↑↓ to navigate, Enter to play, Esc to go back)", playlist.name)
        } else {
            "🎵 Playlist Tracks".to_string()
        };

        let track_items: Vec<ListItem> = self.state.selected_playlist_tracks
            .iter()
            .map(|track| {
                let artists = track.artists.iter()
                    .map(|a| a.name.clone())
                    .collect::<Vec<String>>()
                    .join(", ");
                ListItem::new(format!("{} - {}", track.name, artists))
            })
            .collect();

        let tracks_list = List::new(track_items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_stateful_widget(tracks_list, area, &mut self.list_state);
    }

    fn render_queue(&mut self, f: &mut Frame, area: Rect) {
        // Auto-load queue when entering this view
        if self.state.queue.is_empty() && self.state.user_authenticated {
            // Trigger queue load (this will be async, so display loading message)
            tokio::spawn({
                let client = self.spotify_client.clone();
                async move {
                    if let Some(client) = client {
                        let _ = client.get_queue().await;
                    }
                }
            });
        }

        let queue_items: Vec<ListItem> = if self.state.queue.is_empty() {
            vec![
                ListItem::new("╔══════════════════════════════════════════════╗"),
                ListItem::new("║             🎵 QUEUE IS EMPTY                ║"),
                ListItem::new("╠══════════════════════════════════════════════╣"),
                ListItem::new("║  • Press 'Q' to refresh queue                ║"),
                ListItem::new("║  • Press 'm' on any track to add to queue    ║"),
                ListItem::new("║  • Play songs from Search/Recently Played    ║"),
                ListItem::new("║    to auto-populate similar tracks           ║"),
                ListItem::new("╚══════════════════════════════════════════════╝"),
            ]
        } else {
            let mut items = vec![];

            // Calculate maximum song name length to determine column width first
            let base_track_width = 33; // minimum width
            let max_track_width = 50; // maximum width to prevent too much expansion
            let mut actual_track_width = base_track_width;

            // Find the longest track name that would need more space
            for track in &self.state.queue {
                let display_len = if track.name.len() > max_track_width - 3 {
                    max_track_width
                } else {
                    track.name.len().max(base_track_width)
                };
                actual_track_width = actual_track_width.max(display_len);
            }

            let artist_width = 20;
            let time_width = 4;

            // Add header section with dynamic widths
            items.push(ListItem::new(""));

            // Simple clean table design
            let track_col_width = actual_track_width + 4; // +4 for "  # " or " ▶ " prefix

            // Build the header without outer borders
            items.push(ListItem::new("              UP NEXT              "));
            items.push(ListItem::new(""));

            let column_header = format!("  # {:<width$} │ {:<artist_width$} │ {:^time_width$}",
                "TRACK NAME",
                "ARTIST",
                "TIME",
                width = actual_track_width,
                artist_width = artist_width,
                time_width = time_width);
            items.push(ListItem::new(column_header));

            // Make separator match the exact column spacing of data rows
            let separator = format!("{}─┼─{}─┼─{}",
                "─".repeat(track_col_width),
                "─".repeat(artist_width),
                "─".repeat(time_width));
            items.push(ListItem::new(separator));

            // Add queue items with dynamic formatting
            for (i, track) in self.state.queue.iter().enumerate() {
                let artist_names: String = track
                    .artists
                    .iter()
                    .map(|a| a.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                let duration_sec = track.duration_ms / 1000;
                let duration_formatted = format!("{}:{:02}", duration_sec / 60, duration_sec % 60);

                // Use dynamic track name formatting - don't truncate unless absolutely necessary
                let track_name = if track.name.len() > actual_track_width {
                    format!("{}...", &track.name[..actual_track_width - 3])
                } else {
                    track.name.clone()
                };

                let artists_display = if artist_names.len() > artist_width {
                    format!("{}...", &artist_names[..artist_width - 3])
                } else {
                    artist_names
                };

                let item_text = if i == 0 {
                    format!(" ▶ {:<width$} │ {:<artist_width$} │ {:>time_width$}",
                           track_name,
                           artists_display,
                           duration_formatted,
                           width = actual_track_width,
                           artist_width = artist_width,
                           time_width = time_width)
                } else {
                    format!("{:>2}. {:<width$} │ {:<artist_width$} │ {:>time_width$}",
                           i + 1,
                           track_name,
                           artists_display,
                           duration_formatted,
                           width = actual_track_width,
                           artist_width = artist_width,
                           time_width = time_width)
                };
                items.push(ListItem::new(item_text));
            }

            // No footer border needed
            items.push(ListItem::new(""));
            let total_tracks = self.state.queue.len();
            let total_duration: u32 = self.state.queue.iter().map(|t| t.duration_ms).sum();
            let total_minutes = (total_duration / 1000) / 60;
            items.push(ListItem::new(format!("📊 {} tracks • ~{} minutes total", total_tracks, total_minutes)));

            items
        };

        let queue_list = List::new(queue_items)
            .block(Block::default().title("🎵 Queue (↑↓ to navigate, Enter to play, Q to refresh)").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        f.render_stateful_widget(queue_list, area, &mut self.list_state);
    }

    fn render_errors(&mut self, f: &mut Frame, area: Rect) {
        let error_items: Vec<ListItem> = if self.state.error_logs.is_empty() {
            vec![
                ListItem::new("No errors or radio logs yet"),
                ListItem::new("Play tracks from search/recently played to see radio logs here"),
            ]
        } else {
            // Show logs in reverse order (newest first)
            self.state.error_logs
                .iter()
                .rev()
                .enumerate()
                .map(|(i, log)| {
                    ListItem::new(format!("{}. {}", i + 1, log))
                })
                .collect()
        };

        let errors_list = List::new(error_items)
            .block(Block::default().title("📻 Radio Logs & Errors (Press '7' to view, newest first)").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        f.render_stateful_widget(errors_list, area, &mut self.list_state);
    }

    fn render_albums(&self, f: &mut Frame, area: Rect) {
        let album_items: Vec<ListItem> = if self.state.user_albums.is_empty() {
            vec![
                ListItem::new("Metallica"),
                ListItem::new("Spartan Metallican Visionary"),
                ListItem::new("Metallicash"),
                ListItem::new("Metallica: Burbón"),
                ListItem::new("Metallica Tribute Band"),
                ListItem::new("Doce Penas do Metallica"),
            ]
        } else {
            self.state.user_albums
                .iter()
                .map(|a| ListItem::new(a.name.clone()))
                .collect()
        };

        let albums_list = List::new(album_items)
            .block(Block::default().title("Artists").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        f.render_widget(albums_list, area);
    }

    fn render_artists(&self, f: &mut Frame, area: Rect) {
        let artist_items: Vec<ListItem> = if self.state.user_artists.is_empty() {
            vec![
                ListItem::new("Metallica Greatest Hits"),
                ListItem::new("Metallica: Essentials"),
                ListItem::new("Metallica: Live"),
                ListItem::new("Metallica: Complete"),
                ListItem::new("METALLICA live"),
                ListItem::new("Metallica: Studio Albums"),
                ListItem::new("Metallica: Live São Paulo '99"),
                ListItem::new("Metallica - Black Album"),
                ListItem::new("Metallica - Whisky in the Jar"),
                ListItem::new("METALLICA Pallavicini"),
                ListItem::new("Metallica 2002 Soliloquy 2019"),
                ListItem::new("Metallica Family Playlist"),
                ListItem::new("Metallica Load / Reload (Good Ones)"),
                ListItem::new("Metallica Chile 15 Abril 2020 - Estadio Nacional"),
            ]
        } else {
            self.state.user_artists
                .iter()
                .map(|a| ListItem::new(a.name.clone()))
                .collect()
        };

        let artists_list = List::new(artist_items)
            .block(Block::default().title("Playlists").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        f.render_widget(artists_list, area);
    }

    fn render_player_detail(&self, f: &mut Frame, area: Rect) {
        let player_text = Paragraph::new("Player Details\n\nTrack information and controls will appear here.")
            .block(Block::default().title("Player").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(player_text, area);
    }

    fn render_player(&self, f: &mut Frame, area: Rect) {
        let player_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ])
            .split(area);

        // Current track info with enhanced details
        let track_info = if let Some(ref track) = self.state.current_track {
            let artist_names: String = track
                .artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<_>>()
                .join(", ");

            let mut info = format!("🎵 {} - {}", track.name, artist_names);

            // Add album info if available
            if let Some(ref album) = track.album {
                info.push_str(&format!("\n💿 Album: {}", album.name));
            }

            // Add playback status and progress if available
            if let Some(ref playback) = self.state.current_playback {
                let status_icon = if playback.is_playing { "▶" } else { "⏸️" };
                info.push_str(&format!("\n{} Status: {}", status_icon,
                    if playback.is_playing { "Playing" } else { "Paused" }));

                // Add progress info - always show something
                if let Some(progress_ms) = playback.progress_ms {
                    let duration_ms = track.duration_ms;
                    let progress_sec = progress_ms / 1000;
                    let duration_sec = duration_ms / 1000;
                    let progress_min = progress_sec / 60;
                    let progress_sec_remainder = progress_sec % 60;
                    let duration_min = duration_sec / 60;
                    let duration_sec_remainder = duration_sec % 60;

                    info.push_str(&format!("\n⏱️  Progress: {}:{:02} / {}:{:02}",
                        progress_min, progress_sec_remainder,
                        duration_min, duration_sec_remainder));

                    // Add progress bar
                    let progress_percentage = (progress_ms as f64 / duration_ms as f64 * 100.0) as u8;
                    let bar_width = 20;
                    let filled = (progress_percentage as f64 / 100.0 * bar_width as f64) as usize;
                    let bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);
                    info.push_str(&format!("\n[{}] {}%", bar, progress_percentage));
                } else {
                    // Show duration even if no progress data
                    let duration_sec = track.duration_ms / 1000;
                    let duration_min = duration_sec / 60;
                    let duration_sec_remainder = duration_sec % 60;
                    info.push_str(&format!("\n⏱️  Duration: {}:{:02} (Progress unavailable)",
                        duration_min, duration_sec_remainder));

                    // Show empty progress bar
                    let bar = "░".repeat(20);
                    info.push_str(&format!("\n[{}] No progress data", bar));
                }

                // Add device info
                info.push_str(&format!("\n🎧 Device: {}", playback.device.name));

                // Add shuffle/repeat status
                if playback.shuffle_state {
                    info.push_str(" 🔀");
                }
                match playback.repeat_state.as_str() {
                    "track" => info.push_str(" 🔂"),
                    "context" => info.push_str(" 🔁"),
                    _ => {}
                }
            } else {
                info.push_str(&format!("\n{} Status: {}",
                    if self.state.is_playing { "▶" } else { "⏸️" },
                    if self.state.is_playing { "Playing" } else { "Paused" }));
            }

            info.push_str(&format!("\n{}",
                if self.state.user_authenticated { "✅ Authenticated" } else { "❌ Not authenticated" }));

            if !self.state.auth_message.is_empty() {
                info.push_str(&format!("\n{}", self.state.auth_message));
            }

            info
        } else {
            format!("No track playing\n{}\n{}",
                if self.state.user_authenticated { "✅ Authenticated for playback" } else { "❌ Press 's' to sync or 'u' to authenticate" },
                if !self.state.auth_message.is_empty() { &self.state.auth_message } else { "" })
        };

        let track_widget = Paragraph::new(track_info)
            .block(Block::default().borders(Borders::ALL).title("Now Playing"));

        f.render_widget(track_widget, player_chunks[0]);

        // Player controls
        let play_status = if self.state.is_playing { "⏸ Pause" } else { "▶ Play" };
        let shuffle_icon = match self.state.shuffle_mode {
            ShuffleMode::Off => "",
            ShuffleMode::On => " 🔀",
            ShuffleMode::SmartShuffle => " 🔀✨",
        };
        let controls = format!("⏮ Prev | {} | Next ⏭{}\n\nControls:\nEnter: Play | m: Add to Queue | s: Sync | P: Shuffle\nSpace: Play/Pause | /: Search | ↑↓: Navigate\nn: Next | p: Previous | Alt+R: Prev | Alt+T: Next | q: Quit\n+/-: Volume | u: Auth | r: Refresh Recent | L: Load Liked Songs | Q: Refresh Queue\n1-7: Switch Views | Ctrl+←→: Switch Tabs (7=Errors/Logs)", play_status, shuffle_icon);
        let controls_color = if self.state.user_authenticated { Color::Green } else { Color::Yellow };
        let controls_widget = Paragraph::new(controls)
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .style(Style::default().fg(controls_color));

        f.render_widget(controls_widget, player_chunks[1]);

        // Volume and status
        let mut status_info = format!("Volume: {}%\nStatus: {}\nMode: {}",
            self.state.volume,
            if self.state.is_playing { "Playing" } else { "Paused" },
            if self.state.user_authenticated { "Premium" } else { "Browse Only" }
        );

        // Add auth message (always show something for testing)
        if !self.state.auth_message.is_empty() {
            status_info.push_str(&format!("\n\nMESSAGE: {}", self.state.auth_message));
        } else {
            status_info.push_str("\n\nTEST: Press 's' to sync");
        }
        let status_widget = Paragraph::new(status_info)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .wrap(Wrap { trim: true });

        f.render_widget(status_widget, player_chunks[2]);
    }
}

pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}