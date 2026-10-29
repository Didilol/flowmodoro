mod commands;
mod db;
mod focus;
mod sound;
mod state;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{thread, time::Duration};

use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

use commands::{AppState, BarSolid, Settings, box_visible, close_box, commit, open_box, show_home, state_view};
use db::Db;
use state::{Core, Snapshot};

const BAR_W: f64 = 400.0;
// Room above the bar for the mascot; the empty part lets clicks through (see BarSolid).
const BAR_H: f64 = 140.0;
const BAR_MARGIN: f64 = 20.0;
const ENTRY_W: f64 = 560.0;
const ENTRY_H: f64 = 460.0;

#[cfg(target_os = "macos")]
const OPEN_LABEL: &str = "Abrir caixa (⌥ Espaço)";
#[cfg(not(target_os = "macos"))]
const OPEN_LABEL: &str = "Abrir caixa (Ctrl Espaço)";
#[cfg(target_os = "macos")]
const SOUND_LABEL: &str = "Sons (⌥ S)";
#[cfg(not(target_os = "macos"))]
const SOUND_LABEL: &str = "Sons (Ctrl Alt S)";
#[cfg(target_os = "macos")]
const HOME_LABEL: &str = "Visão geral do dia (⌥ H)";
#[cfg(not(target_os = "macos"))]
const HOME_LABEL: &str = "Visão geral do dia (Ctrl Alt H)";

fn place_bar(bar: &WebviewWindow) {
    let Ok(Some(monitor)) = bar.primary_monitor() else { return };
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let x = area.position.x + area.size.width as i32 - ((BAR_W + BAR_MARGIN) * scale) as i32;
    let y = area.position.y + area.size.height as i32 - ((BAR_H + BAR_MARGIN) * scale) as i32;
    let _ = bar.set_position(PhysicalPosition::new(x, y));
}

fn save_now(app: &AppHandle) {
    let core = app.state::<AppState>();
    let core = core.0.lock().unwrap();
    app.state::<Db>().save_snapshot(&Snapshot::of(&core));
}

/// ⌥ Space: opens the box when idle, closes it when open, nudges the bar mid-task.
fn on_open_shortcut(app: &AppHandle) {
    if box_visible(app) {
        close_box(app);
        return;
    }
    let st = app.state::<AppState>();
    let core = st.0.lock().unwrap();
    if core.task.is_some() {
        drop(core);
        let _ = app.emit_to("bar", "nudge", ());
        return;
    }
    let view = state_view(&core, &app.state::<Db>());
    drop(core);
    open_box(app, "intent", view);
}

const HOVER_FAST_MS: u64 = 40;
const HOVER_FAR_MS: u64 = 150;
const HOVER_IDLE_MS: u64 = 600;
const HOVER_NEAR_PX: f64 = 220.0;

/// The bar never becomes the active window, and macOS only sends hover to active
/// windows. So the core watches the cursor and tells the bar where it is (in CSS px),
/// or (-1, -1) once it leaves.
fn start_hover_tracking(app: AppHandle) {
    let last: std::sync::Arc<Mutex<Option<(i32, i32)>>> = Default::default();
    let ignoring: std::sync::Arc<Mutex<bool>> = Default::default();
    // Fast only while the cursor is near the bar; next to idle otherwise.
    let delay = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(HOVER_IDLE_MS));
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(delay.load(Ordering::Relaxed)));
            let handle = app.clone();
            let last = last.clone();
            let ignoring = ignoring.clone();
            let delay = delay.clone();
            let _ = app.run_on_main_thread(move || {
                let Some(bar) = handle.get_webview_window("bar") else { return };
                let mut over_solid = false;
                delay.store(HOVER_IDLE_MS, Ordering::Relaxed);
                let inside = (|| {
                    if !bar.is_visible().ok()? {
                        return None;
                    }
                    let cursor = handle.cursor_position().ok()?;
                    let pos = bar.outer_position().ok()?;
                    let size = bar.outer_size().ok()?;
                    let scale = bar.scale_factor().ok()?;
                    let x = cursor.x - pos.x as f64;
                    let y = cursor.y - pos.y as f64;
                    let (w, h) = (size.width as f64, size.height as f64);
                    let near = scale * HOVER_NEAR_PX;
                    let close = x > -near && y > -near && x < w + near && y < h + near;
                    delay.store(if close { HOVER_FAST_MS } else { HOVER_FAR_MS }, Ordering::Relaxed);
                    let within = x >= 0.0 && y >= 0.0 && x < w && y < h;
                    let (cx, cy) = (x / scale, y / scale);
                    over_solid = within && handle.state::<BarSolid>().contains(cx, cy);
                    within.then(|| (cx as i32, cy as i32))
                })();
                // Transparent parts of the bar window let clicks through to whatever is below.
                let mut ignoring = ignoring.lock().unwrap();
                if *ignoring == over_solid {
                    *ignoring = !over_solid;
                    let _ = bar.set_ignore_cursor_events(!over_solid);
                }
                let mut last = last.lock().unwrap();
                if *last == inside {
                    return;
                }
                *last = inside;
                let (x, y) = inside.unwrap_or((-1, -1));
                let _ = handle.emit_to("bar", "pointer", (x, y));
            });
        }
    });
}

/// ⌥ H: the overview is only ever opened on purpose, and the same keys put it away.
fn on_home_shortcut(app: &AppHandle) {
    let Some(w) = app.get_webview_window("home") else { return show_home(app) };
    if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
        let _ = w.hide();
        focus::restore_frontmost();
    } else {
        show_home(app);
    }
}

fn on_task_shortcut(app: &AppHandle, mode: &str) {
    let st = app.state::<AppState>();
    let core = st.0.lock().unwrap();
    if core.task.is_none() || box_visible(app) {
        return;
    }
    let view = state_view(&core, &app.state::<Db>());
    drop(core);
    open_box(app, mode, view);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ⌥ on macOS. On Windows Alt+Space opens the window menu and Alt+letter drives app
    // menus, so Ctrl+Space and Ctrl+Alt+letter there.
    #[cfg(target_os = "macos")]
    let (open, task_mods) = (Shortcut::new(Some(Modifiers::ALT), Code::Space), Modifiers::ALT);
    #[cfg(not(target_os = "macos"))]
    let (open, task_mods) = (
        Shortcut::new(Some(Modifiers::CONTROL), Code::Space),
        Modifiers::CONTROL | Modifiers::ALT,
    );
    let lost = Shortcut::new(Some(task_mods), Code::KeyF);
    let full = Shortcut::new(Some(task_mods), Code::KeyD);
    let home = Shortcut::new(Some(task_mods), Code::KeyH);
    let sounds = Shortcut::new(Some(task_mods), Code::KeyS);

    let shortcuts = tauri_plugin_global_shortcut::Builder::new()
        .with_shortcuts([open, lost, full, home, sounds])
        .expect("atalho global inválido")
        .with_handler(move |app, shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            if shortcut == &open {
                on_open_shortcut(app);
            } else if shortcut == &lost {
                on_task_shortcut(app, "lost");
            } else if shortcut == &full {
                on_task_shortcut(app, "full");
            } else if shortcut == &home {
                on_home_shortcut(app);
            } else if shortcut == &sounds {
                sound::toggle(app);
            }
        })
        .build();

    tauri::Builder::default()
        .plugin(shortcuts)
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::show_box,
            commands::hide_box,
            commands::start_task,
            commands::complete_step,
            commands::set_step,
            commands::log_event,
            commands::park_task,
            commands::drop_task,
            commands::finish_task,
            commands::resume_parked,
            commands::drop_parked,
            commands::add_thought,
            commands::list_thoughts,
            commands::resolve_thought,
            commands::add_ref,
            commands::refs_nudge,
            commands::hide_home_window,
            commands::overview,
            commands::add_plan,
            commands::remove_plan,
            commands::start_plan,
            commands::new_task,
            commands::thought_to_plan,
            commands::open_sounds,
            commands::set_bar_solid,
            commands::get_settings,
        ])
        .setup(|app| {
            // No Dock icon: the app lives in the bar, the tray and behind the shortcut.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db = Db::open(&data_dir.join("flowmodoro.db"))?;
            // Reopen where we left off; the timer stayed paused while the app was closed.
            let core = db.load_snapshot().map(Snapshot::into_core).unwrap_or_else(Core::default);
            let resumed = core.task.is_some();
            if resumed {
                db.event(core.task.as_ref().map(|t| t.session_id), "app_reaberto", None);
            }
            app.manage(db);
            app.manage(AppState(Mutex::new(core)));
            app.manage(BarSolid::default());

            let bar = WebviewWindowBuilder::new(app, "bar", WebviewUrl::App("index.html".into()))
                .title("Flowmodoro")
                .inner_size(BAR_W, BAR_H)
                .resizable(false)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .visible_on_all_workspaces(true)
                .skip_taskbar(true)
                .focusable(false)
                .focused(false)
                .visible(false)
                .build()?;
            place_bar(&bar);
            focus::float_everywhere(&bar);

            let entry = WebviewWindowBuilder::new(app, "entry", WebviewUrl::App("entry.html".into()))
                .title("Flowmodoro")
                .inner_size(ENTRY_W, ENTRY_H)
                .resizable(false)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .visible_on_all_workspaces(true)
                .skip_taskbar(true)
                .center()
                .visible(false)
                .build()?;
            focus::float_everywhere(&entry);

            // Like Spotlight: clicking anywhere else closes the box.
            // Only once it actually had focus, or it would vanish if macOS refuses to activate us.
            let entry_handle = entry.clone();
            let had_focus = AtomicBool::new(false);
            entry.on_window_event(move |event| match event {
                WindowEvent::Focused(true) => had_focus.store(true, Ordering::Relaxed),
                WindowEvent::Focused(false) => {
                    if had_focus.swap(false, Ordering::Relaxed) {
                        let _ = entry_handle.hide();
                    }
                }
                _ => {}
            });

            let open_item = MenuItem::with_id(app, "open", OPEN_LABEL, true, None::<&str>)?;
            let home_item = MenuItem::with_id(app, "home", HOME_LABEL, true, None::<&str>)?;
            let sound_item = MenuItem::with_id(app, "sound", SOUND_LABEL, true, None::<&str>)?;
            let mascot_on = Settings::load(&app.state::<Db>()).mascot;
            let mascot_item = CheckMenuItem::with_id(app, "mascot", "Mascote", true, mascot_on, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Sair do Flowmodoro", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &home_item, &sound_item, &mascot_item, &quit_item])?;
            let mut tray = TrayIconBuilder::new()
                .tooltip("Flowmodoro")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "open" => on_open_shortcut(app),
                    "mascot" => {
                        let on = mascot_item.is_checked().unwrap_or(true);
                        commands::set_mascot(app, on);
                    }
                    "home" => show_home(app),
                    "sound" => sound::show(app),
                    "quit" => {
                        save_now(app);
                        app.exit(0);
                    }
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            focus::start_tracking(app.handle().clone());
            start_hover_tracking(app.handle().clone());

            // Keep the saved timer fresh in case the app is killed without "Sair".
            let handle = app.handle().clone();
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(15));
                    save_now(&handle);
                }
            });

            let handle = app.handle();
            let st = handle.state::<AppState>();
            let core = st.0.lock().unwrap();
            commit(handle, &core, &handle.state::<Db>());
            let view = state_view(&core, &handle.state::<Db>());
            drop(core);
            if !resumed {
                // Show the box on launch so it's obvious the app is running.
                open_box(handle, "intent", view);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Flowmodoro");
}
