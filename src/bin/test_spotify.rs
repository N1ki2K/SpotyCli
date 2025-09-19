use spotycli::api::SpotifyClient;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client_id = env::var("SPOTIFY_CLIENT_ID")
        .unwrap_or_else(|_| "1d0e741dd1054d48b8f0d899a559162e".to_string());
    let client_secret = env::var("SPOTIFY_CLIENT_SECRET")
        .unwrap_or_else(|_| "722052880a3640fa9fcbce480d87f222".to_string());

    println!("🎵 Testing Spotify API connection...");

    let mut spotify_client = SpotifyClient::new(client_id, client_secret);

    match spotify_client.authenticate().await {
        Ok(_) => {
            println!("✅ Successfully authenticated with Spotify!");

            // Test search functionality
            println!("🔍 Testing search for 'Metallica'...");
            match spotify_client.search("Metallica", "track", 5).await {
                Ok(results) => {
                    if let Some(tracks) = results.tracks {
                        println!("✅ Found {} tracks:", tracks.items.len());
                        for (i, track) in tracks.items.iter().enumerate() {
                            let artists = track.artists
                                .iter()
                                .map(|a| a.name.clone())
                                .collect::<Vec<_>>()
                                .join(", ");
                            println!("  {}. {} - {}", i + 1, track.name, artists);
                        }
                    } else {
                        println!("❌ No tracks found in search results");
                    }
                }
                Err(e) => {
                    println!("❌ Search failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Authentication failed: {}", e);
        }
    }

    Ok(())
}