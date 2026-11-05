// Automatic updates from GitHub Releases. Checks in the background, downloads quietly and
// installs only while nothing is being timed, so an update never cuts into a focus session.
// After installing, the app restarts and reopens where it left off.

use std::{thread, time::Duration};

use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::commands::{AppState, box_visible};

const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const IDLE_POLL: Duration = Duration::from_secs(60);

pub fn start(app: AppHandle) {
    // `tauri dev` would otherwise replace itself with the published build.
    if cfg!(debug_assertions) {
        return;
    }
    thread::spawn(move || {
        loop {
            if let Err(e) = tauri::async_runtime::block_on(check_and_install(&app)) {
                eprintln!("atualização: {e}");
            }
            thread::sleep(CHECK_EVERY);
        }
    });
}

/// No timer running and no conversation open in the box.
fn idle(app: &AppHandle) -> bool {
    let timing = {
        let st = app.state::<AppState>();
        let core = st.0.lock().unwrap();
        core.task.as_ref().is_some_and(|t| !t.paused)
    };
    !timing && !box_visible(app)
}

async fn check_and_install(app: &AppHandle) -> tauri_plugin_updater::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };
    let bytes = update.download(|_, _| {}, || {}).await?;
    while !idle(app) {
        thread::sleep(IDLE_POLL);
    }
    crate::save_now(app);
    update.install(bytes)?;
    app.restart();
}
