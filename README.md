# SpotyCli - Terminal Spotify Client

A terminal-based Spotify client built in Rust using the Spotify Web API.

## Features

- Search for tracks, albums, artists, and playlists
- Browse your library and playlists
- Terminal-based UI with keyboard navigation
- Real-time music controls

## Setup

1. **Get Spotify API Credentials**
   - Go to [Spotify Developer Dashboard](https://developer.spotify.com/dashboard/)
   - Create a new app
   - Copy your Client ID and Client Secret

2. **Set Environment Variables**
   ```bash
   export SPOTIFY_CLIENT_ID="your_client_id_here"
   export SPOTIFY_CLIENT_SECRET="your_client_secret_here"
   ```

   Or create a `.env` file (copy from `.env.example`):
   ```
   SPOTIFY_CLIENT_ID=your_client_id_here
   SPOTIFY_CLIENT_SECRET=your_client_secret_here
   ```

3. **Build and Run**
   ```bash
   cargo build --release
   cargo run
   ```

## Controls

### Navigation
- `q` - Quit application
- `/` - Enter search mode
- `Enter` - Execute search / Play selected track
- `Esc` - Exit search mode
- `↑/↓` - Navigate lists
- `1-5` - Switch between views:
  - `1` - Search
  - `2` - Library
  - `3` - Playlists
  - `4` - Albums
  - `5` - Artists

### Playback Controls (Requires Spotify Premium)

**To Enable Playback Features:**
1. First, authenticate by running: `cargo run --bin authenticate`
2. Follow the browser authentication flow
3. Run the main app: `cargo run`
4. Playback controls will now be available!

**Playback Controls:**
- `u` - Show authentication status / Get auth instructions
- `Space` - Play/Pause current track
- `n` - Next track
- `p` - Previous track
- `+` - Volume up
- `-` - Volume down
- `Enter` - Play selected track

**Requirements:**
- **Spotify Premium** account (required for playback)
- Active Spotify device (desktop app, mobile, web player)
- Successful authentication via browser

## Screenshots

The application features a three-panel layout similar to the Spotify desktop client:
- Left sidebar: Library and playlists
- Main content: Search results, albums, tracks
- Bottom panel: Now playing and controls

## Development

Built with:
- [Ratatui](https://ratatui.rs/) for terminal UI
- [Tokio](https://tokio.rs/) for async runtime
- [Reqwest](https://github.com/seanmonstar/reqwest) for HTTP requests
- [Serde](https://serde.rs/) for JSON serialization

## License

MIT