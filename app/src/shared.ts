export type Kind = "normal" | "watch" | "practice" | "refs";
export type Step = { text: string; kind: Kind; refsCount: number; refsStartedMs: number; nudged: boolean };
export type Tutorial = { position: string; until: string | null };
export type TaskView = {
  sessionId: number;
  title: string;
  step: Step;
  tutorial: Tutorial | null;
  hasQueue: boolean;
  elapsedMs: number;
};
export type StateView = {
  task: TaskView | null;
  parkedTitle: string | null;
  parkedTutorial: boolean;
  parkedPosition: string | null;
  drawerCount: number;
};
export type Thought = { id: number; text: string; createdAt: number };

export const FIVE_MIN = 5 * 60 * 1000;
export const REFS_GOAL = 5;
export const REFS_NUDGE_MS = 15 * 60 * 1000;

export function formatClock(ms: number): string {
  const t = Math.floor(ms / 1000);
  const h = Math.floor(t / 3600);
  const m = Math.floor((t % 3600) / 60);
  const s = String(t % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

export function formatHuman(ms: number): string {
  const m = Math.round(ms / 60000);
  if (m < 1) return "menos de 1 min";
  if (m < 60) return `${m} min`;
  return `${Math.floor(m / 60)} h ${m % 60} min`;
}

/** Restarts a CSS animation class on an element. */
export function replay(el: HTMLElement, cls: string) {
  el.classList.remove(cls);
  void el.offsetWidth;
  el.classList.add(cls);
}

export function escapeHtml(s: string): string {
  return s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);
}

export type Settings = { mascot: boolean };
