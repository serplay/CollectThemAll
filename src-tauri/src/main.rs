// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Cybersecurity studies note: this is the real program entry point (the `main`
// function every Rust binary starts from). We keep it tiny on purpose — all it
// does is hand control over to the library crate's `run()`. Keeping the entry
// point minimal makes the "attack surface" of the binary easy to reason about:
// there is basically nothing here to get wrong.
fn main() {
    cta_collectthemall_lib::run()
}
