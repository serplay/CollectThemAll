// Cybersecurity / architecture note: turning SSR off means there is NO server
// rendering our pages — everything runs locally inside the Tauri desktop app as a
// single-page app. That shrinks the attack surface a lot: there is no web server
// of ours sitting on the internet to attack. The trade-off is that all logic ships
// to the client, which is exactly why the *sensitive* work stays in the Rust backend.
//
// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
export const ssr = false;
