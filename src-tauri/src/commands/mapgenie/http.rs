//! Everything about talking to the network: building the HTTP client and
//! downloading single files.
//!
//! Cybersecurity note: notice the User-Agent string below. A User-Agent is a
//! header that tells the server "what kind of program am I?". We deliberately
//! set a normal browser one so the MapGenie server treats us like a regular
//! visitor. (In a real security course this is also how we learn that headers
//! can be spoofed and therefore should never be *trusted* by a server.)

use reqwest::Client;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::fs;

/// Max simultaneous tile downloads. Tiles are tiny so we can fan out fairly wide
/// without overwhelming the CDN; this is the main lever for download speed.
///
/// (This is also basic "be a polite client" etiquette — we do not hammer the
/// server with unlimited parallel requests, which would look like a tiny DoS.)
pub const TILE_DOWNLOAD_CONCURRENCY: usize = 16;

/// Shared HTTP client for on-demand tile fetches by the protocol handler.
/// `OnceLock` means "build it the first time it is needed, then reuse it" — a
/// thread-safe lazy singleton.
static TILE_CLIENT: OnceLock<Client> = OnceLock::new();

/// Builds a reqwest client pretending to be a normal desktop Chrome browser.
pub fn build_client() -> Result<Client, String> {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())
}

/// Returns the shared tile client, building it on first use.
pub fn tile_client() -> &'static Client {
    TILE_CLIENT.get_or_init(|| build_client().expect("failed to build tile HTTP client"))
}

/// Downloads one URL to one file path.
///
/// Important detail: if the file already exists we return early and do NOT
/// re-download it. This is both a speed optimisation and a small safety net —
/// we never overwrite something we already trusted enough to save.
pub async fn download_file(client: &Client, url: &str, path: PathBuf) -> Result<(), String> {
    if fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(());
    }
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    // Always check the HTTP status before trusting the body. A 404/500 is not data.
    if !resp.status().is_success() {
        return Err(format!("HTTP {} for {}", resp.status(), url));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    fs::write(&path, bytes).await.map_err(|e| e.to_string())?;
    Ok(())
}
