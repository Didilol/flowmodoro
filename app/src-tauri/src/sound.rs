//! The "Sons" window: myNoise itself, trimmed and restyled by an injected skin.
//!
//! It is a remote page, so it gets no access to the app's commands. Closing only
//! hides it, which keeps the sound playing.

use tauri::{AppHandle, Manager, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

use crate::db::Db;
use crate::focus;

const W: f64 = 560.0;
const H: f64 = 660.0;
const DEFAULT_URL: &str = "https://mynoise.net/NoiseMachines/stormSoundGenerator.php";
const LAST_URL_KEY: &str = "sound_url";
const SKIN: &str = include_str!("sound_skin.js");

fn is_mynoise(url: &tauri::Url) -> bool {
    url.scheme() == "https" && url.host_str().is_some_and(|h| h == "mynoise.net" || h.ends_with(".mynoise.net"))
}

/// Brings the window forward. Activation has to happen on the main thread, and this
/// can be called from an async command.
fn bring_forward(app: &AppHandle, w: WebviewWindow) {
    let _ = w.show();
    let _ = app.run_on_main_thread(move || {
        focus::activate_self();
        let _ = w.set_focus();
    });
}

/// Created on first use, so nothing loads or plays until asked.
pub fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("sound") {
        bring_forward(app, w);
        return;
    }
    let db = app.state::<Db>();
    let url = db
        .setting(LAST_URL_KEY)
        .and_then(|u| u.parse::<tauri::Url>().ok())
        .filter(is_mynoise)
        .unwrap_or_else(|| DEFAULT_URL.parse().unwrap());

    let handle = app.clone();
    let built = WebviewWindowBuilder::new(app, "sound", WebviewUrl::External(url))
        .title("Flowmodoro · Sons")
        .inner_size(W, H)
        .min_inner_size(420.0, 480.0)
        .theme(Some(Theme::Dark))
        .center()
        .initialization_script(SKIN)
        // Stay on myNoise; remember the last generator for next time.
        .on_navigation(move |url| {
            if !is_mynoise(url) {
                return false;
            }
            if url.path().starts_with("/NoiseMachines/") {
                handle.state::<Db>().set_setting(LAST_URL_KEY, url.as_str());
            }
            true
        })
        .build();
    let window = match built {
        Ok(w) => w,
        Err(e) => return eprintln!("[sons] janela não abriu: {e}"),
    };
    let w = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = w.hide();
            focus::restore_frontmost();
        }
    });
    bring_forward(app, window);
}

/// ⌥ S: open, or put away if it's already in front. The sound keeps playing either way.
pub fn toggle(app: &AppHandle) {
    match app.get_webview_window("sound") {
        Some(w) if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) => {
            let _ = w.hide();
            focus::restore_frontmost();
        }
        _ => show(app),
    }
}
