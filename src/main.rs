mod api;
mod models;
mod ui;

use anyhow::Result;
use std::env;
use tokio;

use api::SpotifyClient;
use ui::{setup_terminal, restore_terminal, App};

#[tokio::main]
async fn main() -> Result<()> {
    // Get Spotify credentials from environment variables
    let client_id = env::var("SPOTIFY_CLIENT_ID")
        .unwrap_or_else(|_| "your_client_id_here".to_string());
    let client_secret = env::var("SPOTIFY_CLIENT_SECRET")
        .unwrap_or_else(|_| "your_client_secret_here".to_string());

    // Initialize Spotify client
    let mut spotify_client = SpotifyClient::new(client_id, client_secret);

    // Try to authenticate
    if let Err(e) = spotify_client.authenticate().await {
        eprintln!("Failed to authenticate with Spotify: {}", e);
        eprintln!("Please set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET environment variables");
        eprintln!("You can get these from: https://developer.spotify.com/dashboard/");
        return Ok(());
    }

    println!("Successfully authenticated with Spotify!");

    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create and run the app
    let mut app = App::new();
    app.set_spotify_client(spotify_client);
    let result = app.run(&mut terminal);

    // Restore terminal
    restore_terminal(&mut terminal)?;

    if let Err(err) = result {
        eprintln!("Application error: {}", err);
    }

    Ok(())
}
