//! What the app knows right now: the current task and the one parked task.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub const REFS_GOAL: u32 = 5;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Normal,
    /// Tutorial: watch until a given minute.
    Watch,
    /// Tutorial: reproduce what was just watched.
    Practice,
    /// Collect references; ends by count, not by time.
    Refs,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Normal => "normal",
            Kind::Watch => "assistir",
            Kind::Practice => "praticar",
            Kind::Refs => "referencias",
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub text: String,
    pub kind: Kind,
    pub refs_count: u32,
    /// Task time when the reference hunt started; the 15-minute nudge counts from here.
    pub refs_started_ms: u64,
    pub nudged: bool,
}

impl Step {
    pub fn new(text: String, kind: Kind, task_elapsed: u64) -> Self {
        Self { text, kind, refs_count: 0, refs_started_ms: task_elapsed, nudged: false }
    }

    /// A plain step that mentions references turns into a counter.
    pub fn detect(text: String, task_elapsed: u64) -> Self {
        let lower = text.to_lowercase();
        let kind = if lower.contains("referênc") || lower.contains("referenc") {
            Kind::Refs
        } else {
            Kind::Normal
        };
        Self::new(text, kind, task_elapsed)
    }

    pub fn watch(until: &str, task_elapsed: u64) -> Self {
        Self::new(format!("Assistir até {until}"), Kind::Watch, task_elapsed)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Tutorial {
    /// Where the user is in the video ("18:40").
    pub position: String,
    /// The minute the current watch step ends at.
    pub until: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub session_id: i64,
    pub step_row: i64,
    pub title: String,
    pub step: Step,
    pub queue: Vec<String>,
    pub tutorial: Option<Tutorial>,
    /// Time accumulated while running. Paused while parked or while the app is closed.
    pub elapsed_ms: u64,
    #[serde(skip)]
    pub running_since: Option<u64>,
}

impl Task {
    pub fn elapsed(&self) -> u64 {
        self.elapsed_ms + self.running_since.map_or(0, |s| now_ms().saturating_sub(s))
    }

    pub fn pause(&mut self) {
        self.elapsed_ms = self.elapsed();
        self.running_since = None;
    }

    pub fn resume(&mut self) {
        if self.running_since.is_none() {
            self.running_since = Some(now_ms());
        }
    }

    /// What gets written to the step log when the step ends.
    pub fn step_marker(&self) -> Option<String> {
        match self.step.kind {
            Kind::Refs => Some(format!("{} de {}", self.step.refs_count, REFS_GOAL)),
            Kind::Watch => self.tutorial.as_ref().and_then(|t| t.until.clone()),
            _ => None,
        }
    }

    pub fn view(&self) -> TaskView {
        TaskView {
            session_id: self.session_id,
            title: self.title.clone(),
            step: self.step.clone(),
            tutorial: self.tutorial.clone(),
            has_queue: !self.queue.is_empty(),
            elapsed_ms: self.elapsed(),
        }
    }
}

#[derive(Default)]
pub struct Core {
    pub task: Option<Task>,
    pub parked: Option<Task>,
}

/// Saved to disk so the app reopens where it was, with the timer paused in between.
#[derive(Serialize, Deserialize, Default)]
pub struct Snapshot {
    pub task: Option<Task>,
    pub parked: Option<Task>,
}

impl Snapshot {
    pub fn of(core: &Core) -> Self {
        let freeze = |t: &Task| {
            let mut t = t.clone();
            t.pause();
            t
        };
        Self { task: core.task.as_ref().map(freeze), parked: core.parked.as_ref().map(freeze) }
    }

    pub fn into_core(self) -> Core {
        let mut task = self.task;
        if let Some(t) = task.as_mut() {
            t.resume();
        }
        Core { task, parked: self.parked }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    pub session_id: i64,
    pub title: String,
    pub step: Step,
    pub tutorial: Option<Tutorial>,
    pub has_queue: bool,
    pub elapsed_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateView {
    pub task: Option<TaskView>,
    pub parked_title: Option<String>,
    pub parked_tutorial: bool,
    /// Video minute of a parked tutorial, for the overview.
    pub parked_position: Option<String>,
    pub drawer_count: usize,
}
