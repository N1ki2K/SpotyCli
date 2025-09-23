# SpotyCli

A command-line interface for Spotify built in Rust.

## Requirements

- Rust
- A Spotify Premium account

## Setup

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/N1ki2K/spotycli.git
    cd spotycli
    ```

2.  **Create a Spotify App:**

    - Go to the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard/).
    - Click "Create an App".
    - Give it a name and description.
    - Edit the settings and add `http://localhost:8888/callback` to the "Redirect URIs".
    - Take note of your `Client ID` and `Client Secret`.

3.  **Set up environment variables:**

    - Create a `.env` file in the root of the project:

      ```bash
      cp .env.example .env
      ```

    - Open the `.env` file and add your Spotify `Client ID` and `Client Secret`:

      ```
      SPOTIFY_CLIENT_ID=your_client_id
      SPOTIFY_CLIENT_SECRET=your_client_secret
      ```

## Usage

1.  **Authenticate with Spotify:**

    Run the following command to authenticate with your Spotify account. This will open a browser window for you to log in and authorize the application.

    ```bash
    cargo run --bin authenticate
    ```

    This will create a `.spotify_tokens` file in the root of the project with your authentication tokens.

2.  **Run the application:**

    ```bash
    cargo run
    ```

## Features

- View and control your Spotify playback.
- Browse your playlists and liked songs.
- Search for tracks, albums, and artists.
- TUI built with `ratatui` and `crossterm`.

## Dependencies

- `dotenv`
- `tokio`
- `reqwest`
- `serde`
- `serde_json`
- `ratatui`
- `crossterm`
- `anyhow`
- `base64`
- `url`
- `chrono`
- `urlencoding`
- `sha2`
- `rand`
- `warp`
- `webbrowser`
