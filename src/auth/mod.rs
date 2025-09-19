use anyhow::{anyhow, Result};
use base64::Engine;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub scope: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    scope: String,
    token_type: String,
}

pub struct SpotifyAuth {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    client: Client,
}

impl SpotifyAuth {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri: "http://127.0.0.1:8888/callback".to_string(),
            client: Client::new(),
        }
    }

    pub async fn authenticate_user(&self) -> Result<UserTokens> {
        // Generate PKCE parameters
        let code_verifier = generate_code_verifier();
        let code_challenge = generate_code_challenge(&code_verifier);
        let state = generate_state();

        // Set up callback server
        let auth_code = Arc::new(Mutex::new(None::<String>));
        let auth_state = Arc::new(Mutex::new(None::<String>));
        let auth_error = Arc::new(Mutex::new(None::<String>));

        let auth_code_filter = auth_code.clone();
        let auth_state_filter = auth_state.clone();
        let auth_error_filter = auth_error.clone();

        let callback = warp::path("callback")
            .and(warp::query::<HashMap<String, String>>())
            .and_then(move |params: HashMap<String, String>| {
                let auth_code = auth_code_filter.clone();
                let auth_state = auth_state_filter.clone();
                let auth_error = auth_error_filter.clone();

                async move {
                    if let Some(error) = params.get("error") {
                        *auth_error.lock().await = Some(error.clone());
                        return Ok::<_, warp::Rejection>(warp::reply::html(
                            "<h1>Authentication Failed</h1><p>You can close this window.</p>",
                        ));
                    }

                    if let (Some(code), Some(returned_state)) = (params.get("code"), params.get("state")) {
                        *auth_code.lock().await = Some(code.clone());
                        *auth_state.lock().await = Some(returned_state.clone());
                        Ok(warp::reply::html(
                            "<h1>Authentication Successful!</h1><p>You can close this window and return to SpotyCli.</p>",
                        ))
                    } else {
                        *auth_error.lock().await = Some("Missing authorization code".to_string());
                        Ok(warp::reply::html(
                            "<h1>Authentication Failed</h1><p>Missing authorization code. You can close this window.</p>",
                        ))
                    }
                }
            });

        let routes = callback;

        // Start server
        let server = warp::serve(routes).run(([127, 0, 0, 1], 8888));
        tokio::spawn(server);

        // Open browser for user authentication
        let auth_url = format!(
            "https://accounts.spotify.com/authorize?{}",
            [
                ("client_id", self.client_id.as_str()),
                ("response_type", "code"),
                ("redirect_uri", &self.redirect_uri),
                ("code_challenge_method", "S256"),
                ("code_challenge", &code_challenge),
                ("state", &state),
                ("scope", "user-read-playback-state user-modify-playback-state user-read-currently-playing streaming user-library-read playlist-read-private playlist-read-collaborative"),
            ]
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&")
        );

        println!("🔐 Opening browser for Spotify authentication...");
        println!("If browser doesn't open automatically, visit: {}", auth_url);

        if let Err(_) = webbrowser::open(&auth_url) {
            println!("❌ Could not open browser automatically.");
            println!("Please manually open: {}", auth_url);
        }

        // Wait for callback
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if let Some(error) = auth_error.lock().await.clone() {
                return Err(anyhow!("Authentication failed: {}", error));
            }

            if let Some(code) = auth_code.lock().await.clone() {
                let returned_state = auth_state.lock().await.clone();
                if returned_state.as_ref() != Some(&state) {
                    return Err(anyhow!("State mismatch in OAuth callback"));
                }

                // Exchange code for tokens
                return self.exchange_code_for_tokens(&code, &code_verifier).await;
            }
        }
    }

    async fn exchange_code_for_tokens(&self, code: &str, code_verifier: &str) -> Result<UserTokens> {
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("redirect_uri", &self.redirect_uri);
        params.insert("client_id", &self.client_id);
        params.insert("code_verifier", code_verifier);

        let auth_string = format!("{}:{}", self.client_id, self.client_secret);
        let encoded = base64::engine::general_purpose::STANDARD.encode(auth_string.as_bytes());

        let response = self
            .client
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", format!("Basic {}", encoded))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let token_response: TokenResponse = response.json().await?;
            Ok(UserTokens {
                access_token: token_response.access_token,
                refresh_token: token_response.refresh_token.unwrap_or_default(),
                expires_in: token_response.expires_in,
                scope: token_response.scope,
            })
        } else {
            let error_text = response.text().await?;
            Err(anyhow!("Token exchange failed: {}", error_text))
        }
    }

    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<UserTokens> {
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token);

        let auth_string = format!("{}:{}", self.client_id, self.client_secret);
        let encoded = base64::engine::general_purpose::STANDARD.encode(auth_string.as_bytes());

        let response = self
            .client
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", format!("Basic {}", encoded))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let token_response: TokenResponse = response.json().await?;
            Ok(UserTokens {
                access_token: token_response.access_token,
                refresh_token: token_response.refresh_token.unwrap_or_else(|| refresh_token.to_string()),
                expires_in: token_response.expires_in,
                scope: token_response.scope,
            })
        } else {
            let error_text = response.text().await?;
            Err(anyhow!("Token refresh failed: {}", error_text))
        }
    }
}

fn generate_code_verifier() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(128)
        .map(char::from)
        .collect()
}

fn generate_code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

fn generate_state() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}