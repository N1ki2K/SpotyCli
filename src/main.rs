pub mod api;
pub mod auth;
pub mod models;
pub mod ui;

use anyhow::Result;
use dotenv::dotenv;
use std::env;
use std::fs;
use tokio;

use api::SpotifyClient;
use auth::{SpotifyAuth, UserTokens};
use ui::{setup_terminal, restore_terminal, App};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Get Spotify credentials from environment variables
    let client_id = env::var("SPOTIFY_CLIENT_ID")
        .unwrap_or_else(|_| "your_client_id_here".to_string());
    let client_secret = env::var("SPOTIFY_CLIENT_SECRET")
        .unwrap_or_else(|_| "your_client_secret_here".to_string());

    // Initialize Spotify client for basic API access
    let mut spotify_client = SpotifyClient::new(client_id.clone(), client_secret.clone());

    // Try to authenticate for basic API access
    if let Err(e) = spotify_client.authenticate().await {
        eprintln!("Failed to authenticate with Spotify: {}", e);
        eprintln!("Please set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET environment variables");
        eprintln!("You can get these from: https://developer.spotify.com/dashboard/");
        return Ok(());
    }

    println!("✅ Successfully authenticated with Spotify API!");

    // Check for saved authentication tokens
    let user_authenticated = if let Ok(tokens_data) = fs::read_to_string(".spotify_tokens") {
        if let Ok(user_tokens) = serde_json::from_str::<UserTokens>(&tokens_data) {
            spotify_client.set_user_tokens(user_tokens);
            println!("🔑 Found saved authentication tokens!");
            println!("🎵 Playback features are available!");
            true
        } else {
            false
        }
    } else {
        false
    };

    if !user_authenticated {
        println!("💡 Run 'cargo run --bin authenticate' to enable playback features!");
    }

    println!("🎵 Starting SpotyCli...");

    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create and run the app
    let mut app = App::new();
    app.set_spotify_client(spotify_client);

    // Create auth client for user authentication
    let auth_client = SpotifyAuth::new(client_id, client_secret);
    app.set_auth_client(auth_client);

    // Set authentication status if tokens were loaded
    app.state.user_authenticated = user_authenticated;

    let result = app.run(&mut terminal);

    // Restore terminal
    restore_terminal(&mut terminal)?;

    if let Err(err) = result {
        eprintln!("Application error: {}", err);
    }

    Ok(())
}
