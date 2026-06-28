//! Small string/URL helpers used while scraping MapGenie pages.
//!
//! Cybersecurity note: string handling is where a LOT of real bugs live (think
//! of all the injection attacks we studied). These helpers stay deliberately
//! small and predictable, and each one validates its assumptions instead of
//! blindly trusting that a URL or HTML blob is shaped the way we expect.

/// Extracts the JSON value of a named JavaScript variable from raw HTML.
/// Handles both `window.VAR = {...}` and `const VAR = {...}` patterns.
/// Uses serde_json's streaming deserializer so it reads exactly one valid JSON
/// value and stops, safely ignoring the trailing semicolon/script content.
pub fn extract_named_var_json(html: &str, prefix: &str) -> Result<serde_json::Value, String> {
    let start = html
        .find(prefix)
        .ok_or_else(|| format!("'{}' not found in page", prefix.trim()))?
        + prefix.len();
    let slice = &html[start..];
    serde_json::Deserializer::from_str(slice)
        .into_iter::<serde_json::Value>()
        .next()
        .ok_or_else(|| "Failed to parse JSON after variable prefix".to_string())?
        .map_err(|e| e.to_string())
}

/// Turns a possibly-relative URL into an absolute https URL.
///   - "//host/x"  -> "https://host/x"
///   - "/x"        -> "https://<default_host>/x"
///   - otherwise it is already absolute and returned unchanged.
pub fn normalize_url(url: &str, default_host: &str) -> String {
    if url.starts_with("//") {
        format!("https:{}", url)
    } else if url.starts_with('/') {
        format!("https://{}{}", default_host, url)
    } else {
        url.to_string()
    }
}

/// Removes a retina density suffix like "@2x" or "@3x" from a filename in a URL,
/// e.g. ".../sprite@2x.png" -> ".../sprite.png". We only strip it when it really
/// looks like "@<digits>x", so we do not accidentally mangle other URLs.
pub fn strip_density_suffix(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(dot_pos) = url[at_pos..].find('.') {
            let after_at = &url[at_pos + 1..at_pos + dot_pos];
            if after_at.ends_with('x')
                && after_at[..after_at.len() - 1]
                    .chars()
                    .all(|c| c.is_ascii_digit())
            {
                return format!("{}{}", &url[..at_pos], &url[at_pos + dot_pos..]);
            }
        }
    }
    url.to_string()
}

/// Given a sprite-sheet URL, work out the URL of the markers.json that sits next
/// to it in the same folder.
pub fn sibling_markers_json_url(sprite_url: &str) -> Result<String, String> {
    // Strip query string first
    let url_no_query = sprite_url.split('?').next().unwrap_or(sprite_url);
    // Strip @2x/@1x density suffix if present
    let url_clean = strip_density_suffix(url_no_query);
    let last_slash = url_clean
        .rfind('/')
        .ok_or_else(|| format!("Unexpected marker sprite URL format: {}", sprite_url))?;
    Ok(format!("{}/markers.json", &url_clean[..last_slash]))
}
