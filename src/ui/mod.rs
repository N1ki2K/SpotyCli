use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame, Terminal,
};
use std::io;

use crate::models::{AppState, ViewType};
use crate::api::SpotifyClient;

pub struct App {
    pub state: AppState,
    pub list_state: ListState,
    pub input_mode: bool,
    pub spotify_client: Option<SpotifyClient>,
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
        }
    }

    pub fn set_spotify_client(&mut self, client: SpotifyClient) {
        self.spotify_client = Some(client);
    }

    fn trigger_search(&mut self) {
        // For now, just clear any existing results to show we're "searching"
        // In a real implementation, this would spawn an async task
        self.state.search_results = None;
        self.list_state.select(Some(0));
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
                                // Here we would trigger search, but since we're in sync context
                                // we'll just show placeholder results
                                self.trigger_search();
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
                                    '1' => self.state.current_view = ViewType::Search,
                                    '2' => self.state.current_view = ViewType::Library,
                                    '3' => self.state.current_view = ViewType::Playlists,
                                    '4' => self.state.current_view = ViewType::Albums,
                                    '5' => self.state.current_view = ViewType::Artists,
                                    ' ' => {
                                        self.state.is_playing = !self.state.is_playing;
                                    }
                                    _ => {}
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
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
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
            format!("{}\n{}", track.name, artist_names)
        } else {
            "Vivo Alvaro Alexandre · OneBox Pro | Shuffle: Off | Repeat: Off | Volume: 108%.".to_string()
        };

        let track_widget = Paragraph::new(track_info)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(track_widget, player_chunks[0]);

        // Player controls
        let play_status = if self.state.is_playing { "⏸" } else { "▶" };
        let controls = format!("⏮ {} ⏭", play_status);
        let controls_widget = Paragraph::new(controls)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Green));

        f.render_widget(controls_widget, player_chunks[1]);

        // Progress and volume
        let progress = "0:56/04 (-3:07)";
        let progress_widget = Paragraph::new(progress)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(progress_widget, player_chunks[2]);
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