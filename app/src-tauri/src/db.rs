//! Local SQLite file: the focus log (sessions, steps, events), the thought drawer,
//! and a snapshot of the live state.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::state::{Snapshot, now_ms};

/// Unreviewed thoughts disappear on their own after this.
const DRAWER_MAX_AGE_MS: u64 = 7 * 24 * 3600 * 1000;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sessions (
  id INTEGER PRIMARY KEY,
  title TEXT NOT NULL,
  tutorial INTEGER NOT NULL DEFAULT 0,
  started_at INTEGER NOT NULL,
  ended_at INTEGER,
  outcome TEXT,          -- concluida | guardada | soltou
  elapsed_ms INTEGER
);
CREATE TABLE IF NOT EXISTS steps (
  id INTEGER PRIMARY KEY,
  session_id INTEGER NOT NULL REFERENCES sessions(id),
  text TEXT NOT NULL,
  kind TEXT NOT NULL,
  marker TEXT,           -- video minute or reference count
  started_at INTEGER NOT NULL,
  ended_at INTEGER
);
CREATE TABLE IF NOT EXISTS events (
  id INTEGER PRIMARY KEY,
  session_id INTEGER,
  kind TEXT NOT NULL,
  detail TEXT,
  at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS thoughts (
  id INTEGER PRIMARY KEY,
  text TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  resolved_at INTEGER,
  outcome TEXT           -- virou_tarefa | apagado | expirou
);
CREATE TABLE IF NOT EXISTS plan (
  id INTEGER PRIMARY KEY,
  text TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  resolved_at INTEGER,
  outcome TEXT           -- comecou | removido
);
-- Live state ('state') and small settings, one JSON or text value per key.
CREATE TABLE IF NOT EXISTS snapshot (
  key TEXT PRIMARY KEY,
  json TEXT NOT NULL
);
";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thought {
    pub id: i64,
    pub text: String,
    pub created_at: u64,
}

/// An item on the "pra hoje" list. It stays until started or removed.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub id: i64,
    pub text: String,
    pub created_at: u64,
}

/// A task finished in the requested time range, with how long each step took.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoneItem {
    #[serde(skip)]
    pub id: i64,
    pub title: String,
    pub tutorial: bool,
    pub started_at: u64,
    pub ended_at: u64,
    pub elapsed_ms: u64,
    pub steps: Vec<DoneStep>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoneStep {
    pub text: String,
    pub kind: String,
    pub marker: Option<String>,
    pub ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySummary {
    pub done: Vec<DoneItem>,
    /// Time on every task that ended in the range, finished or not.
    pub focus_ms: u64,
    pub steps_done: u32,
}

pub struct Db(Mutex<Connection>);

/// Logging must never take the app down; report and carry on.
fn warn<T>(result: rusqlite::Result<T>) -> Option<T> {
    result.map_err(|e| eprintln!("[db] {e}")).ok()
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self(Mutex::new(conn)))
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap()
    }

    pub fn start_session(&self, title: &str, tutorial: bool) -> i64 {
        let c = self.conn();
        warn(c.execute(
            "INSERT INTO sessions (title, tutorial, started_at) VALUES (?1, ?2, ?3)",
            params![title, tutorial, now_ms() as i64],
        ));
        c.last_insert_rowid()
    }

    pub fn end_session(&self, id: i64, outcome: &str, elapsed_ms: u64) {
        warn(self.conn().execute(
            "UPDATE sessions SET ended_at = ?1, outcome = ?2, elapsed_ms = ?3 WHERE id = ?4",
            params![now_ms() as i64, outcome, elapsed_ms as i64, id],
        ));
    }

    pub fn begin_step(&self, session_id: i64, text: &str, kind: &str) -> i64 {
        let c = self.conn();
        warn(c.execute(
            "INSERT INTO steps (session_id, text, kind, started_at) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, text, kind, now_ms() as i64],
        ));
        c.last_insert_rowid()
    }

    /// Idempotent: a step that already ended keeps its first end time.
    pub fn end_step(&self, id: i64, marker: Option<&str>) {
        warn(self.conn().execute(
            "UPDATE steps SET ended_at = ?1, marker = COALESCE(?2, marker) WHERE id = ?3 AND ended_at IS NULL",
            params![now_ms() as i64, marker, id],
        ));
    }

    pub fn event(&self, session_id: Option<i64>, kind: &str, detail: Option<&str>) {
        warn(self.conn().execute(
            "INSERT INTO events (session_id, kind, detail, at) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, kind, detail, now_ms() as i64],
        ));
    }

    pub fn add_thought(&self, text: &str) -> Thought {
        let created_at = now_ms();
        let c = self.conn();
        warn(c.execute(
            "INSERT INTO thoughts (text, created_at) VALUES (?1, ?2)",
            params![text, created_at as i64],
        ));
        Thought { id: c.last_insert_rowid(), text: text.to_string(), created_at }
    }

    pub fn open_thoughts(&self) -> Vec<Thought> {
        let c = self.conn();
        let now = now_ms();
        warn(c.execute(
            "UPDATE thoughts SET resolved_at = ?1, outcome = 'expirou' WHERE resolved_at IS NULL AND created_at < ?2",
            params![now as i64, now.saturating_sub(DRAWER_MAX_AGE_MS) as i64],
        ));
        let Some(mut stmt) = warn(c.prepare(
            "SELECT id, text, created_at FROM thoughts WHERE resolved_at IS NULL ORDER BY created_at",
        )) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(Thought { id: r.get(0)?, text: r.get(1)?, created_at: r.get::<_, i64>(2)? as u64 })
        });
        warn(rows).map(|rows| rows.filter_map(Result::ok).collect()).unwrap_or_default()
    }

    /// Marks a thought as handled and returns its text.
    pub fn resolve_thought(&self, id: i64, outcome: &str) -> Option<String> {
        let c = self.conn();
        let text: Option<String> = warn(
            c.query_row(
                "SELECT text FROM thoughts WHERE id = ?1 AND resolved_at IS NULL",
                [id],
                |r| r.get(0),
            )
            .optional(),
        )
        .flatten();
        if text.is_some() {
            warn(c.execute(
                "UPDATE thoughts SET resolved_at = ?1, outcome = ?2 WHERE id = ?3",
                params![now_ms() as i64, outcome, id],
            ));
        }
        text
    }

    pub fn add_plan(&self, text: &str) -> PlanItem {
        let created_at = now_ms();
        let c = self.conn();
        warn(c.execute(
            "INSERT INTO plan (text, created_at) VALUES (?1, ?2)",
            params![text, created_at as i64],
        ));
        PlanItem { id: c.last_insert_rowid(), text: text.to_string(), created_at }
    }

    pub fn open_plan(&self) -> Vec<PlanItem> {
        let c = self.conn();
        let Some(mut stmt) = warn(c.prepare(
            "SELECT id, text, created_at FROM plan WHERE resolved_at IS NULL ORDER BY created_at",
        )) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(PlanItem { id: r.get(0)?, text: r.get(1)?, created_at: r.get::<_, i64>(2)? as u64 })
        });
        warn(rows).map(|rows| rows.filter_map(Result::ok).collect()).unwrap_or_default()
    }

    pub fn plan_text(&self, id: i64) -> Option<String> {
        warn(
            self.conn()
                .query_row("SELECT text FROM plan WHERE id = ?1 AND resolved_at IS NULL", [id], |r| r.get(0))
                .optional(),
        )
        .flatten()
    }

    pub fn resolve_plan(&self, id: i64, outcome: &str) {
        warn(self.conn().execute(
            "UPDATE plan SET resolved_at = ?1, outcome = ?2 WHERE id = ?3 AND resolved_at IS NULL",
            params![now_ms() as i64, outcome, id],
        ));
    }

    /// What got done between two instants (the frontend knows the local day bounds).
    pub fn day(&self, from: u64, to: u64) -> DaySummary {
        let c = self.conn();
        let range = params![from as i64, to as i64];
        let mut done = warn(c.prepare(
            "SELECT id, title, tutorial, started_at, ended_at, COALESCE(elapsed_ms, 0)
             FROM sessions
             WHERE outcome = 'concluida' AND ended_at >= ?1 AND ended_at < ?2
             ORDER BY ended_at",
        ))
        .and_then(|mut stmt| {
            let rows = stmt.query_map(range, |r| {
                Ok(DoneItem {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    tutorial: r.get(2)?,
                    started_at: r.get::<_, i64>(3)? as u64,
                    ended_at: r.get::<_, i64>(4)? as u64,
                    elapsed_ms: r.get::<_, i64>(5)? as u64,
                    steps: Vec::new(),
                })
            });
            warn(rows).map(|rows| rows.filter_map(Result::ok).collect::<Vec<_>>())
        })
        .unwrap_or_default();
        // A resumed step gets a new row, so the same text can appear twice: add them up.
        if let Some(mut stmt) = warn(c.prepare(
            "SELECT text, kind, MAX(marker), SUM(ended_at - started_at)
             FROM steps WHERE session_id = ?1 AND ended_at IS NOT NULL
             GROUP BY text, kind ORDER BY MIN(started_at)",
        )) {
            for item in &mut done {
                let rows = stmt.query_map([item.id], |r| {
                    Ok(DoneStep {
                        text: r.get(0)?,
                        kind: r.get(1)?,
                        marker: r.get(2)?,
                        ms: r.get::<_, i64>(3)?.max(0) as u64,
                    })
                });
                item.steps = warn(rows).map(|rows| rows.filter_map(Result::ok).collect()).unwrap_or_default();
            }
        }
        let focus_ms: i64 = warn(c.query_row(
            "SELECT COALESCE(SUM(elapsed_ms), 0) FROM sessions WHERE ended_at >= ?1 AND ended_at < ?2",
            range,
            |r| r.get(0),
        ))
        .unwrap_or(0);
        let steps_done = warn(c.query_row(
            "SELECT COUNT(*) FROM events WHERE kind = 'passo_concluido' AND at >= ?1 AND at < ?2",
            range,
            |r| r.get(0),
        ))
        .unwrap_or(0);
        DaySummary { done, focus_ms: focus_ms as u64, steps_done }
    }

    pub fn setting(&self, key: &str) -> Option<String> {
        warn(
            self.conn()
                .query_row("SELECT json FROM snapshot WHERE key = ?1", [key], |r| r.get(0))
                .optional(),
        )
        .flatten()
    }

    pub fn set_setting(&self, key: &str, value: &str) {
        warn(self.conn().execute(
            "INSERT OR REPLACE INTO snapshot (key, json) VALUES (?1, ?2)",
            params![key, value],
        ));
    }

    pub fn save_snapshot(&self, snapshot: &Snapshot) {
        let Ok(json) = serde_json::to_string(snapshot) else { return };
        warn(self.conn().execute(
            "INSERT OR REPLACE INTO snapshot (key, json) VALUES ('state', ?1)",
            [json],
        ));
    }

    pub fn load_snapshot(&self) -> Option<Snapshot> {
        let json: String = warn(
            self.conn()
                .query_row("SELECT json FROM snapshot WHERE key = 'state'", [], |r| r.get(0))
                .optional(),
        )
        .flatten()?;
        serde_json::from_str(&json).map_err(|e| eprintln!("[db] snapshot ilegível: {e}")).ok()
    }
}
