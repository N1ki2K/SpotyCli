pub mod api;
pub mod auth;
pub mod models;
pub mod ui;

use anyhow::Result;
use dotenv::dotenv;
use std::env;
use std::fs;
use std::io;
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
    println!("💡 Press 'u' within 5 seconds to check Spotify devices, or wait to continue...");

    // Give user 5 seconds to press 'u' for device check
    use crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        terminal::{enable_raw_mode, disable_raw_mode},
    };
    use std::time::{Duration, Instant};

    let start_time = Instant::now();
    let timeout = Duration::from_secs(5);
    let mut should_check_devices = false;

    // Enable raw mode for input detection
    enable_raw_mode()?;

    while start_time.elapsed() < timeout {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('u') {
                    should_check_devices = true;
                    break;
                }
            }
        }

        let remaining = timeout.saturating_sub(start_time.elapsed()).as_secs();
        print!("\r⏱️  Device check in {} seconds (press 'u' now)...  ", remaining);
        io::Write::flush(&mut io::stdout())?;
    }

    disable_raw_mode()?;
    println!("\r                                                    \r"); // Clear the countdown line

    // If user pressed 'u', check devices before starting
    if should_check_devices {
        println!("🔍 Checking Spotify devices...");

        let mut temp_client = SpotifyClient::new(client_id.clone(), client_secret.clone());
        temp_client.authenticate().await?;

        if user_authenticated {
            if let Ok(tokens_data) = std::fs::read_to_string(".spotify_tokens") {
                if let Ok(user_tokens) = serde_json::from_str::<UserTokens>(&tokens_data) {
                    temp_client.set_user_tokens(user_tokens);

                    match temp_client.get_available_devices().await {
                        Ok(devices) => {
                            if devices.devices.is_empty() {
                                println!("❌ No Spotify devices found!");
                                println!("💡 Would you like me to launch Spotify in the background? (y/n)");

                                let mut input = String::new();
                                io::stdin().read_line(&mut input)?;

                                if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                                    match SpotifyClient::launch_spotify_background() {
                                        Ok(_) => {
                                            println!("⏳ Waiting for Spotify to start...");
                                            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

                                            // Check devices again after launching
                                            match temp_client.get_available_devices().await {
                                                Ok(new_devices) => {
                                                    if new_devices.devices.is_empty() {
                                                        println!("⚠️  Spotify launched but no devices detected yet. Try starting playback in Spotify.");
                                                    } else {
                                                        println!("✅ Found {} Spotify device(s) after launch:", new_devices.devices.len());
                                                        for device in &new_devices.devices {
                                                            let status = if device.is_active { "🔊 ACTIVE" } else { "⏸️  Inactive" };
                                                            println!("   - {} ({}): {}", device.name, device.device_type, status);
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    println!("❌ Failed to check devices after launch: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            println!("❌ Failed to launch Spotify: {}", e);
                                            println!("💡 Please manually open Spotify app and start playing something.");
                                        }
                                    }
                                } else {
                                    println!("💡 Please manually open Spotify app (desktop, mobile, or web) and start playing something.");
                                }
                            } else {
                                println!("✅ Found {} Spotify device(s):", devices.devices.len());
                                for device in &devices.devices {
                                    let status = if device.is_active { "🔊 ACTIVE" } else { "⏸️  Inactive" };
                                    println!("   - {} ({}): {}", device.name, device.device_type, status);
                                }

                                let active_count = devices.devices.iter().filter(|d| d.is_active).count();
                                if active_count == 0 {
                                    println!("⚠️  No devices are currently active. Start playing something in Spotify first.");
                                }
                            }
                        },
                        Err(e) => {
                            println!("❌ Failed to check devices: {}", e);
                        }
                    }
                }
            }
        } else {
            println!("❌ Not authenticated for playback. Run: cargo run --bin authenticate");
        }

        println!("Press Enter to continue...");
        io::stdin().read_line(&mut String::new())?;
    }

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

    // Auto-load playlists and liked songs if user is authenticated
    if user_authenticated {
        app.load_user_playlists().await;
        app.load_recently_played_from_spotify().await;
        app.load_liked_songs().await;
        // Sync current playback state
        app.sync_playback_state().await;
    }

    let result = app.run(&mut terminal).await;

    // Restore terminal
    restore_terminal(&mut terminal)?;

    if let Err(err) = result {
        eprintln!("Application error: {}", err);
    }

    Ok(())
}
