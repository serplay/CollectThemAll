//! The `commands` module groups everything the frontend is allowed to call.
//!
//! Cybersecurity note: in Tauri, the JavaScript frontend can only invoke the
//! specific Rust functions we explicitly mark as `#[tauri::command]` and register
//! in lib.rs. That allow-list is a trust boundary — the UI cannot run arbitrary
//! Rust, only the doors we open for it. Keeping all those doors inside one
//! `commands` area makes that boundary easy to find and review.

pub mod mapgenie;