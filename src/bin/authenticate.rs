use spotycli::auth::SpotifyAuth;
use anyhow::Result;
use dotenv::dotenv;
use std::env;
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    let client_id = env::var("SPOTIFY_CLIENT_ID")
        .expect("SPOTIFY_CLIENT_ID must be set");
    let client_secret = env::var("SPOTIFY_CLIENT_SECRET")
        .expect("SPOTIFY_CLIENT_SECRET must be set");

    println!("🔐 SpotyCli Authentication");
    println!("This will authenticate you with Spotify for playback features.");
    println!("You need a Spotify Premium account for music playback.\n");

    let auth_client = SpotifyAuth::new(client_id, client_secret);

    match auth_client.authenticate_user().await {
        Ok(tokens) => {
            println!("✅ Authentication successful!");
            println!("🎵 You can now use playback features in SpotyCli!");

            // Save tokens to a file for the main app to use
            let tokens_json = serde_json::to_string_pretty(&tokens)?;
            fs::write(".spotify_tokens", tokens_json)?;
            println!("🔑 Tokens saved. Run 'cargo run' to use SpotyCli with playback!");
        }
        Err(e) => {
            println!("❌ Authentication failed: {}", e);
            println!("Make sure you have a Spotify Premium account and try again.");
        }
    }

    Ok(())
}