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

use crate::models::{AppState, ViewType};
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

    fn trigger_search(&mut self) {
        // For now, just clear any existing results to show we're "searching"
        // In a real implementation, this would spawn an async task
        self.state.search_results = None;
        self.list_state.select(Some(0));
    }

    fn authenticate_user(&mut self) {
        if self.state.user_authenticated {
            // Already authenticated, show status
            self.state.auth_message = "✅ Already authenticated for playback!".to_string();
        } else if self.auth_client.is_some() {
            // Show authentication instructions
            self.state.auth_message = "🔐 Authentication required! Exit app (press 'q') and run: cargo run --bin authenticate".to_string();
        } else {
            self.state.auth_message = "❌ Authentication client not available".to_string();
        }
    }

    fn toggle_playback(&mut self) {
        if self.state.user_authenticated {
            self.state.is_playing = !self.state.is_playing;
            // In a real implementation, this would call the Spotify API
        }
    }

    fn play_selected_track(&mut self) {
        if !self.state.user_authenticated {
            return;
        }

        if let Some(selected) = self.list_state.selected() {
            if let Some(ref results) = self.state.search_results {
                if let Some(ref tracks) = results.tracks {
                    if let Some(track) = tracks.items.get(selected) {
                        self.state.current_track = Some(track.clone());
                        self.state.is_playing = true;
                        // In a real implementation, this would call spotify_client.play_track(&track.uri)
                    }
                }
            }
        }
    }

    fn next_track(&mut self) {
        if self.state.user_authenticated {
            // In a real implementation, this would call spotify_client.next_track()
        }
    }

    fn previous_track(&mut self) {
        if self.state.user_authenticated {
            // In a real implementation, this would call spotify_client.previous_track()
        }
    }

    fn volume_up(&mut self) {
        if self.state.user_authenticated {
            self.state.volume = (self.state.volume + 10).min(100);
            // In a real implementation, this would call spotify_client.set_volume()
        }
    }

    fn volume_down(&mut self) {
        if self.state.user_authenticated {
            self.state.volume = self.state.volume.saturating_sub(10);
            // In a real implementation, this would call spotify_client.set_volume()
        }
    }

    pub fn run<B: ratatui::backend::Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('/') => {
                            self.input_mode = true;
                            self.state.current_view = ViewType::Search;
                        }
                        KeyCode::Enter => {
                            if self.input_mode {
                                self.input_mode = false;
                                self.trigger_search();
                            } else {
                                self.play_selected_track();
                            }
                        }
                        KeyCode::Esc => {
                            self.input_mode = false;
                        }
                        KeyCode::Char(c) => {
                            if self.input_mode {
                                self.state.search_query.push(c);
                            } else {
                                match c {
                                    '1' => {
                                        self.state.current_view = ViewType::Search;
                                        self.state.auth_message.clear();
                                    }
                                    '2' => {
                                        self.state.current_view = ViewType::Library;
                                        self.state.auth_message.clear();
                                    }
                                    '3' => {
                                        self.state.current_view = ViewType::Playlists;
                                        self.state.auth_message.clear();
                                    }
                                    '4' => {
                                        self.state.current_view = ViewType::Albums;
                                        self.state.auth_message.clear();
                                    }
                                    '5' => {
                                        self.state.current_view = ViewType::Artists;
                                        self.state.auth_message.clear();
                                    }
                                    ' ' => {
                                        self.toggle_playback();
                                        self.state.auth_message.clear();
                                    }
                                    'u' => {
                                        self.authenticate_user();
                                    }
                                    'n' => {
                                        self.next_track();
                                        self.state.auth_message.clear();
                                    }
                                    'p' => {
                                        self.previous_track();
                                        self.state.auth_message.clear();
                                    }
                                    '+' => {
                                        self.volume_up();
                                        self.state.auth_message.clear();
                                    }
                                    '-' => {
                                        self.volume_down();
                                        self.state.auth_message.clear();
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
                        KeyCode::Up => {
                            self.move_selection(-1);
                        }
                        KeyCode::Down => {
                            self.move_selection(1);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn move_selection(&mut self, direction: i32) {
        if !self.input_mode {
            let len = self.get_current_list_len();
            if len > 0 {
                let current = self.list_state.selected().unwrap_or(0);
                let new_index = if direction > 0 {
                    (current + 1) % len
                } else {
                    if current == 0 { len - 1 } else { current - 1 }
                };
                self.list_state.select(Some(new_index));
            }
        }
    }

    fn get_current_list_len(&self) -> usize {
        match self.state.current_view {
            ViewType::Search => {
                if let Some(ref results) = self.state.search_results {
                    if let Some(ref tracks) = results.tracks {
                        tracks.items.len()
                    } else { 0 }
                } else { 0 }
            }
            ViewType::Playlists => self.state.user_playlists.len(),
            ViewType::Albums => self.state.user_albums.len(),
            ViewType::Artists => self.state.user_artists.len(),
            _ => 0,
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

        // Library section
        let library_items = vec![
            ListItem::new("Recently Played"),
            ListItem::new("Liked Songs"),
            ListItem::new("Albums"),
            ListItem::new("Artists"),
            ListItem::new("Podcasts"),
        ];

        let library_list = List::new(library_items)
            .block(Block::default().title("Library").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

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
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_widget(playlists_list, sidebar_chunks[2]);
    }

    fn render_main_content(&mut self, f: &mut Frame, area: Rect) {
        match self.state.current_view {
            ViewType::Search => self.render_search(f, area),
            ViewType::Library => self.render_library(f, area),
            ViewType::Playlists => self.render_playlists(f, area),
            ViewType::Albums => self.render_albums(f, area),
            ViewType::Artists => self.render_artists(f, area),
            ViewType::Player => self.render_player_detail(f, area),
        }
    }

    fn render_search(&mut self, f: &mut Frame, area: Rect) {
        let search_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

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

        // Search results
        if let Some(ref results) = self.state.search_results {
            if let Some(ref tracks) = results.tracks {
                let track_items: Vec<ListItem> = tracks
                    .items
                    .iter()
                    .map(|track| {
                        let artist_names: String = track
                            .artists
                            .iter()
                            .map(|a| a.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ");

                        ListItem::new(format!("{} - {}", track.name, artist_names))
                    })
                    .collect();

                let tracks_list = List::new(track_items)
                    .block(Block::default().title("Songs").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

                f.render_stateful_widget(tracks_list, search_chunks[1], &mut self.list_state);
            }
        } else {
            // Show sample search results like in the image
            let sample_tracks = vec![
                ListItem::new("Nothing Else Matters - Metallica"),
                ListItem::new("Master Of Puppets - Metallica"),
                ListItem::new("One - Metallica"),
                ListItem::new("For Whom The Bell Tolls - Remastered - Metallica"),
                ListItem::new("Enter Sandman - Metallica"),
                ListItem::new("The Unforgiven - Metallica"),
                ListItem::new("Sad But True - Metallica"),
                ListItem::new("Fade To Black - Remastered - Metallica"),
                ListItem::new("Seek & Destroy - Remastered - Metallica"),
                ListItem::new("Wherever I May Roam - Metallica"),
                ListItem::new("Battery - Metallica"),
                ListItem::new("Hardwired - Metallica"),
            ];

            let tracks_list = List::new(sample_tracks)
                .block(Block::default().title("Songs").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

            f.render_stateful_widget(tracks_list, search_chunks[1], &mut self.list_state);
        }
    }

    fn render_library(&self, f: &mut Frame, area: Rect) {
        let library_text = Paragraph::new("Your Library\n\nRecently played tracks and saved music will appear here.")
            .block(Block::default().title("Library").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(library_text, area);
    }

    fn render_playlists(&self, f: &mut Frame, area: Rect) {
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
            .block(Block::default().title("Albums").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_widget(playlists_list, area);
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
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

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
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

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

        // Current track info
        let track_info = if let Some(ref track) = self.state.current_track {
            let artist_names: String = track
                .artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            format!("🎵 {} - {}\n{}\n{}", track.name, artist_names,
                if self.state.user_authenticated { "✅ Authenticated" } else { "❌ Not authenticated" },
                if !self.state.auth_message.is_empty() { &self.state.auth_message } else { "" })
        } else {
            format!("No track playing\n{}\n{}",
                if self.state.user_authenticated { "✅ Authenticated for playback" } else { "❌ Press 'u' to authenticate" },
                if !self.state.auth_message.is_empty() { &self.state.auth_message } else { "" })
        };

        let track_widget = Paragraph::new(track_info)
            .block(Block::default().borders(Borders::ALL).title("Now Playing"));

        f.render_widget(track_widget, player_chunks[0]);

        // Player controls
        let play_status = if self.state.is_playing { "⏸ Pause" } else { "▶ Play" };
        let controls = format!("⏮ Prev | {} | Next ⏭\n\nControls:\nSpace: Play/Pause\nn: Next | p: Previous\n+/-: Volume | u: Auth", play_status);
        let controls_color = if self.state.user_authenticated { Color::Green } else { Color::Yellow };
        let controls_widget = Paragraph::new(controls)
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .style(Style::default().fg(controls_color));

        f.render_widget(controls_widget, player_chunks[1]);

        // Volume and status
        let status_info = format!("Volume: {}%\nStatus: {}\nMode: {}",
            self.state.volume,
            if self.state.is_playing { "Playing" } else { "Paused" },
            if self.state.user_authenticated { "Premium" } else { "Browse Only" }
        );
        let status_widget = Paragraph::new(status_info)
            .block(Block::default().borders(Borders::ALL).title("Status"));

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