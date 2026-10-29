//! Everything the two windows can ask the core to do.
//!
//! The bar only shows the current step and sends button presses. Every
//! conversation (questions, thought dump, breathing, drawer) happens in the
//! entry box, the one window allowed to take the keyboard. The overview ("home")
//! is a look at the day on request; starting anything from it goes through the box.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::db::{DaySummary, Db, PlanItem, Thought};
use crate::focus;
use crate::sound;
use crate::state::{Core, Kind, REFS_GOAL, Snapshot, StateView, Step, Task, TaskView, Tutorial};

pub struct AppState(pub Mutex<Core>);

/// The painted parts of the bar window (bar and mascot), in CSS px. Everything else lets clicks through.
#[derive(Default)]
pub struct BarSolid(Mutex<Vec<[f64; 4]>>);

impl BarSolid {
    /// Until the bar has reported its shape, the whole window counts as solid.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let rects = self.0.lock().unwrap();
        rects.is_empty() || rects.iter().any(|[l, t, w, h]| x >= *l && x < l + w && y >= *t && y < t + h)
    }
}

type Res<T = ()> = Result<T, String>;

#[derive(Clone, Serialize)]
struct BoxMode {
    mode: String,
    state: StateView,
    /// Title to start with, when the box opens for an item from the "pra hoje" list.
    title: Option<String>,
    plan_id: Option<i64>,
}

#[derive(Serialize)]
pub struct Overview {
    day: DaySummary,
    plan: Vec<PlanItem>,
    thoughts: Vec<Thought>,
}

pub fn state_view(core: &Core, db: &Db) -> StateView {
    StateView {
        task: core.task.as_ref().map(Task::view),
        parked_title: core.parked.as_ref().map(|t| t.title.clone()),
        parked_tutorial: core.parked.as_ref().is_some_and(|t| t.tutorial.is_some()),
        parked_position: core.parked.as_ref().and_then(|t| t.tutorial.as_ref()).map(|t| t.position.clone()),
        drawer_count: db.open_thoughts().len(),
    }
}

/// Saves, tells both windows, and shows the bar only while there is a task.
pub fn commit(app: &AppHandle, core: &Core, db: &Db) {
    db.save_snapshot(&Snapshot::of(core));
    let _ = app.emit("state", state_view(core, db));
    if let Some(bar) = app.get_webview_window("bar") {
        let _ = if core.task.is_some() { bar.show() } else { bar.hide() };
    }
}

pub fn open_box(app: &AppHandle, mode: &str, state: StateView) {
    open_box_with(app, mode, state, None, None);
}

fn open_box_with(app: &AppHandle, mode: &str, state: StateView, title: Option<String>, plan_id: Option<i64>) {
    focus::remember_frontmost();
    // The bar listens too, so the mascot can react.
    let _ = app.emit("box-mode", BoxMode { mode: mode.to_string(), state, title, plan_id });
    if let Some(w) = app.get_webview_window("entry") {
        let _ = w.center();
        let _ = w.show();
        focus::activate_self();
        let _ = w.set_focus();
    }
}

pub fn close_box(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("entry") {
        let _ = w.hide();
    }
    focus::restore_frontmost();
}

const HOME_W: f64 = 860.0;
const HOME_H: f64 = 680.0;

/// Built on first use, so it costs nothing until asked for; closing it only hides it.
pub fn show_home(app: &AppHandle) {
    let w = match app.get_webview_window("home") {
        Some(w) => w,
        None => {
            #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
            let mut builder = WebviewWindowBuilder::new(app, "home", WebviewUrl::App("home.html".into()))
                .title("Flowmodoro · Visão geral")
                .inner_size(HOME_W, HOME_H)
                .min_inner_size(420.0, 480.0)
                .center();
            #[cfg(target_os = "macos")]
            {
                builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
            }
            let w = match builder.build() {
                Ok(w) => w,
                Err(e) => return eprintln!("[visão geral] janela não abriu: {e}"),
            };
            let handle = w.clone();
            w.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = handle.hide();
                }
            });
            w
        }
    };
    let _ = w.show();
    focus::activate_self();
    let _ = w.set_focus();
}

/// Once there is something to do, the overview gets out of the way.
fn hide_home(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("home") {
        let _ = w.hide();
    }
}

pub fn box_visible(app: &AppHandle) -> bool {
    app.get_webview_window("entry")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

fn clean(s: String) -> Option<String> {
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn end_current(db: &Db, task: &Task) {
    db.end_step(task.step_row, task.step_marker().as_deref());
}

fn begin(db: &Db, task: &mut Task, step: Step) {
    task.step = step;
    task.step_row = db.begin_step(task.session_id, &task.step.text, task.step.kind.as_str());
    db.event(Some(task.session_id), "passo_iniciado", Some(&task.step.text));
}

#[tauri::command]
pub fn get_state(st: State<AppState>, db: State<Db>) -> StateView {
    state_view(&st.0.lock().unwrap(), &db)
}

/// From the bar: ⚑ ("lost") or the waves button ("full").
#[tauri::command]
pub fn show_box(app: AppHandle, st: State<AppState>, db: State<Db>, mode: String) {
    let core = st.0.lock().unwrap();
    if core.task.is_none() || !matches!(mode.as_str(), "lost" | "full") {
        return;
    }
    let view = state_view(&core, &db);
    drop(core);
    open_box(&app, &mode, view);
}

#[tauri::command]
pub fn hide_box(app: AppHandle) {
    close_box(&app);
}

#[tauri::command]
pub fn start_task(
    app: AppHandle,
    st: State<AppState>,
    db: State<Db>,
    title: String,
    steps: Vec<String>,
    watch_until: Option<String>,
    plan_id: Option<i64>,
) -> Res {
    let title = clean(title).ok_or("Escreva a tarefa.")?;
    let mut queue: Vec<String> = steps.into_iter().filter_map(clean).collect();
    let watch_until = watch_until.and_then(clean);
    let step = match &watch_until {
        Some(until) => Step::watch(until, 0),
        None if queue.is_empty() => return Err("A tarefa precisa de pelo menos um passo.".into()),
        None => Step::detect(queue.remove(0), 0),
    };
    let tutorial = watch_until.map(|until| Tutorial { position: "0:00".into(), until: Some(until) });

    let mut core = st.0.lock().unwrap();
    if let Some(old) = core.task.take() {
        end_current(&db, &old);
        db.end_session(old.session_id, "soltou", old.elapsed());
    }
    let session_id = db.start_session(&title, tutorial.is_some());
    db.event(Some(session_id), "sessao_iniciada", Some(&title));
    if let Some(id) = plan_id {
        db.resolve_plan(id, "comecou");
        db.event(Some(session_id), "plano_comecou", Some(&title));
    }
    let mut task = Task {
        session_id,
        step_row: 0,
        title,
        step: step.clone(),
        queue,
        tutorial,
        elapsed_ms: 0,
        running_since: None,
    };
    begin(&db, &mut task, step);
    task.resume();
    core.task = Some(task);
    commit(&app, &core, &db);
    drop(core);
    hide_home(&app);
    close_box(&app);
    Ok(())
}

/// ✓ on the bar. Moves on by itself when it can, otherwise asks in the box.
#[tauri::command]
pub fn complete_step(app: AppHandle, st: State<AppState>, db: State<Db>) {
    let mut core = st.0.lock().unwrap();
    let Some(task) = core.task.as_mut() else { return };
    end_current(&db, task);
    db.event(Some(task.session_id), "passo_concluido", Some(&task.step.text));

    let ask = match task.step.kind {
        Kind::Watch => {
            if let Some(t) = task.tutorial.as_mut() {
                if let Some(until) = t.until.take() {
                    t.position = until;
                }
            }
            Some("practice")
        }
        Kind::Practice => Some("watchUntil"),
        _ if !task.queue.is_empty() => {
            let next = task.queue.remove(0);
            let elapsed = task.elapsed();
            begin(&db, task, Step::detect(next, elapsed));
            None
        }
        _ if task.tutorial.is_some() => Some("watchUntil"),
        _ => Some("next"),
    };
    commit(&app, &core, &db);
    let view = state_view(&core, &db);
    drop(core);
    match ask {
        Some(mode) => open_box(&app, mode, view),
        None => close_box(&app),
    }
}

/// Sets the current step: "E agora?", a smaller step, practice, or the next watch chunk.
#[tauri::command]
pub fn set_step(
    app: AppHandle,
    st: State<AppState>,
    db: State<Db>,
    text: Option<String>,
    kind: Kind,
    until: Option<String>,
    reason: Option<String>,
) -> Res {
    let text = text.and_then(clean);
    let mut core = st.0.lock().unwrap();
    let Some(task) = core.task.as_mut() else { return Ok(()) };
    let elapsed = task.elapsed();
    let step = match kind {
        Kind::Watch => {
            let until = until.and_then(clean).ok_or("Diga até que minuto.")?;
            end_current(&db, task);
            if let Some(t) = task.tutorial.as_mut() {
                t.until = Some(until.clone());
            }
            Step::watch(&until, elapsed)
        }
        Kind::Practice => {
            end_current(&db, task);
            Step::new(text.unwrap_or_else(|| "Reproduzir no AE o que viu".into()), kind, elapsed)
        }
        _ => {
            let text = text.ok_or("Escreva o passo.")?;
            end_current(&db, task);
            Step::detect(text, elapsed)
        }
    };
    begin(&db, task, step);
    if let Some(reason) = reason {
        db.event(Some(task.session_id), &reason, Some(&task.step.text));
    }
    commit(&app, &core, &db);
    drop(core);
    close_box(&app);
    Ok(())
}

#[tauri::command]
pub fn log_event(st: State<AppState>, db: State<Db>, kind: String, detail: Option<String>) {
    let session = st.0.lock().unwrap().task.as_ref().map(|t| t.session_id);
    db.event(session, &kind, detail.as_deref());
}

/// "Guardar para depois": only one parked task; parking another lets go of the old one.
#[tauri::command]
pub fn park_task(app: AppHandle, st: State<AppState>, db: State<Db>, position: Option<String>) {
    let mut core = st.0.lock().unwrap();
    let Some(mut task) = core.task.take() else { return };
    end_current(&db, &task);
    task.pause();
    if let (Some(t), Some(pos)) = (task.tutorial.as_mut(), position.and_then(clean)) {
        db.event(Some(task.session_id), "parou_em", Some(&pos));
        t.position = pos;
    }
    if let Some(old) = core.parked.take() {
        db.end_session(old.session_id, "soltou", old.elapsed_ms);
        db.event(Some(old.session_id), "soltou", Some(&old.title));
    }
    db.event(Some(task.session_id), "guardou", Some(&task.title));
    core.parked = Some(task);
    commit(&app, &core, &db);
}

#[tauri::command]
pub fn drop_task(app: AppHandle, st: State<AppState>, db: State<Db>) {
    let mut core = st.0.lock().unwrap();
    let Some(task) = core.task.take() else { return };
    end_current(&db, &task);
    db.end_session(task.session_id, "soltou", task.elapsed());
    db.event(Some(task.session_id), "soltou", Some(&task.title));
    commit(&app, &core, &db);
}

/// Ends the task and returns it one last time so the box can say "Feito".
#[tauri::command]
pub fn finish_task(app: AppHandle, st: State<AppState>, db: State<Db>) -> Option<TaskView> {
    let mut core = st.0.lock().unwrap();
    let task = core.task.take()?;
    end_current(&db, &task);
    let view = task.view();
    db.end_session(task.session_id, "concluida", view.elapsed_ms);
    db.event(Some(task.session_id), "tarefa_concluida", Some(&task.title));
    commit(&app, &core, &db);
    Some(view)
}

#[tauri::command]
pub fn resume_parked(app: AppHandle, st: State<AppState>, db: State<Db>) -> Option<TaskView> {
    let mut core = st.0.lock().unwrap();
    let mut task = core.parked.take()?;
    if let Some(old) = core.task.take() {
        end_current(&db, &old);
        db.end_session(old.session_id, "soltou", old.elapsed());
    }
    db.event(Some(task.session_id), "retomou", Some(&task.title));
    let step = task.step.clone();
    begin(&db, &mut task, step);
    task.resume();
    let view = task.view();
    core.task = Some(task);
    commit(&app, &core, &db);
    drop(core);
    hide_home(&app);
    Some(view)
}

#[tauri::command]
pub fn drop_parked(app: AppHandle, st: State<AppState>, db: State<Db>) {
    let mut core = st.0.lock().unwrap();
    let Some(task) = core.parked.take() else { return };
    db.end_session(task.session_id, "soltou", task.elapsed_ms);
    db.event(Some(task.session_id), "soltou", Some(&task.title));
    commit(&app, &core, &db);
}

#[tauri::command]
pub fn add_thought(app: AppHandle, st: State<AppState>, db: State<Db>, text: String) -> Res<Thought> {
    let text = clean(text).ok_or("Escreva o pensamento.")?;
    let core = st.0.lock().unwrap();
    let thought = db.add_thought(&text);
    db.event(core.task.as_ref().map(|t| t.session_id), "pensamento_anotado", Some(&text));
    commit(&app, &core, &db);
    Ok(thought)
}

#[tauri::command]
pub fn list_thoughts(db: State<Db>) -> Vec<Thought> {
    db.open_thoughts()
}

/// The drawer has only two ways out: become a task, or be deleted.
#[tauri::command]
pub fn resolve_thought(
    app: AppHandle,
    st: State<AppState>,
    db: State<Db>,
    id: i64,
    outcome: String,
) -> Res<Option<String>> {
    let event = match outcome.as_str() {
        "virou_tarefa" => "pensamento_virou_tarefa",
        "apagado" => "pensamento_apagado",
        _ => return Err("Destino inválido.".into()),
    };
    let core = st.0.lock().unwrap();
    let text = db.resolve_thought(id, &outcome);
    db.event(core.task.as_ref().map(|t| t.session_id), event, text.as_deref());
    commit(&app, &core, &db);
    Ok(text)
}

/// "+" on the bar while collecting references.
#[tauri::command]
pub fn add_ref(app: AppHandle, st: State<AppState>, db: State<Db>) {
    let mut core = st.0.lock().unwrap();
    let Some(task) = core.task.as_mut() else { return };
    if task.step.kind != Kind::Refs {
        return;
    }
    task.step.refs_count += 1;
    let count = task.step.refs_count;
    db.event(Some(task.session_id), "ref_salva", Some(&format!("{count} de {REFS_GOAL}")));
    commit(&app, &core, &db);
    let view = state_view(&core, &db);
    drop(core);
    if count >= REFS_GOAL {
        open_box(&app, "refsDone", view);
    } else {
        focus::restore_frontmost();
    }
}

/// Sent by the bar once the reference hunt passes 15 minutes.
#[tauri::command]
pub fn refs_nudge(app: AppHandle, st: State<AppState>, db: State<Db>) {
    let mut core = st.0.lock().unwrap();
    let Some(task) = core.task.as_mut() else { return };
    if task.step.kind != Kind::Refs || task.step.nudged || task.step.refs_count >= REFS_GOAL {
        return;
    }
    task.step.nudged = true;
    let detail = format!("{} de {REFS_GOAL}", task.step.refs_count);
    db.event(Some(task.session_id), "refs_aviso_15min", Some(&detail));
    commit(&app, &core, &db);
    let view = state_view(&core, &db);
    drop(core);
    open_box(&app, "refsNudge", view);
}

// ---------- overview ----------

#[tauri::command]
pub fn hide_home_window(app: AppHandle) {
    hide_home(&app);
    focus::restore_frontmost();
}

/// `from`/`to` are the local day bounds, computed by the window.
#[tauri::command]
pub fn overview(db: State<Db>, from: u64, to: u64) -> Overview {
    Overview { day: db.day(from, to), plan: db.open_plan(), thoughts: db.open_thoughts() }
}

#[tauri::command]
pub fn add_plan(app: AppHandle, st: State<AppState>, db: State<Db>, text: String) -> Res<PlanItem> {
    let text = clean(text).ok_or("Escreva o que é pra hoje.")?;
    let core = st.0.lock().unwrap();
    let item = db.add_plan(&text);
    db.event(core.task.as_ref().map(|t| t.session_id), "plano_adicionado", Some(&text));
    commit(&app, &core, &db);
    Ok(item)
}

#[tauri::command]
pub fn remove_plan(app: AppHandle, st: State<AppState>, db: State<Db>, id: i64) {
    let core = st.0.lock().unwrap();
    let text = db.plan_text(id);
    db.resolve_plan(id, "removido");
    db.event(core.task.as_ref().map(|t| t.session_id), "plano_removido", text.as_deref());
    commit(&app, &core, &db);
}

/// "Começar" on a "pra hoje" item. The item leaves the list only once the task really starts.
#[tauri::command]
pub fn start_plan(app: AppHandle, st: State<AppState>, db: State<Db>, id: i64) {
    let Some(text) = db.plan_text(id) else { return };
    let core = st.0.lock().unwrap();
    // Mid-task, the current one has to be parked or dropped first.
    let mode = if core.task.is_some() { "planSwitch" } else { "plan" };
    let view = state_view(&core, &db);
    drop(core);
    open_box_with(&app, mode, view, Some(text), Some(id));
}

/// "Começar algo" from the overview when nothing is running.
#[tauri::command]
pub fn new_task(app: AppHandle, st: State<AppState>, db: State<Db>) {
    let core = st.0.lock().unwrap();
    if core.task.is_some() {
        return;
    }
    let view = state_view(&core, &db);
    drop(core);
    open_box(&app, "intent", view);
}

/// A drawer thought moved onto the "pra hoje" list.
#[tauri::command]
pub fn thought_to_plan(app: AppHandle, st: State<AppState>, db: State<Db>, id: i64) {
    let core = st.0.lock().unwrap();
    if let Some(text) = db.resolve_thought(id, "virou_tarefa") {
        db.add_plan(&text);
        db.event(core.task.as_ref().map(|t| t.session_id), "pensamento_pra_hoje", Some(&text));
    }
    commit(&app, &core, &db);
}

/// "Sons" from the overview. Async so the window can be built off the main thread.
#[tauri::command]
pub async fn open_sounds(app: AppHandle) {
    sound::show(&app);
}

#[tauri::command]
pub fn set_bar_solid(solid: State<BarSolid>, rects: Vec<[f64; 4]>) {
    *solid.0.lock().unwrap() = rects;
}

// ---------- settings ----------

#[derive(Clone, Serialize)]
pub struct Settings {
    /// Off saves the little it costs: no drawing, no blinking.
    pub mascot: bool,
}

impl Settings {
    pub fn load(db: &Db) -> Self {
        Settings { mascot: db.setting("mascot").as_deref() != Some("off") }
    }
}

#[tauri::command]
pub fn get_settings(db: State<Db>) -> Settings {
    Settings::load(&db)
}

/// From the tray's "Mascote" checkbox.
pub fn set_mascot(app: &AppHandle, on: bool) {
    let db = app.state::<Db>();
    db.set_setting("mascot", if on { "on" } else { "off" });
    let _ = app.emit("settings", Settings::load(&db));
}
