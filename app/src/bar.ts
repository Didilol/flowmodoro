import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";
import { Mascot, type Moment } from "./mascot";
import { FIVE_MIN, REFS_GOAL, REFS_NUDGE_MS, formatClock, replay, type Settings, type StateView, type TaskView } from "./shared";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const bar = $("bar");
const step = $("step");
const kind = $("kind");
const stepText = $("stepText");
const stepSub = $("stepSub");
const time = $("time");
const fill = $("fill");
const wrap = document.querySelector<HTMLElement>(".bar-wrap")!;
const dialWrap = $("dialWrap");
const dial = $("dial");
const dialTime = $("dialTime");
const dialFill = $("dialFill");
const pauseBtn = $("pauseBtn");
const mascotEl = $("mascot");
const mascot = new Mascot(mascotEl, true);

const KIND_LABEL = { normal: "", watch: "Tutorial · assistir", practice: "Tutorial · praticar", refs: "Referências" };

let task: TaskView | null = null;
// Rust owns the clock; we only extrapolate from its last snapshot.
let snapshotAt = 0;
// One "5 minutes" log entry per session, and one nudge request per reference hunt.
const flowLogged = new Set<number>();
let nudgeAsked = false;
// Circle and minimized timer: only the circle shows until the pointer is over it.
let mode: Settings["timer"] = "bar";
let compact = false;
let open = false;
let closeTimer: number | undefined;
// Minimized: the circle shrinks to a dot with no time, and grows back now and then.
const PEEK_MS = 8000;
let peekMin = 15;
let peeking = false;
let peekTimer: number | undefined;
// Which peek interval the clock is in; null until the first tick, so starting never peeks.
let lastPeek: number | null = null;

function apply(state: StateView) {
  const prev = task;
  task = state.task;
  if (!task) {
    mascot.setBase("idle");
    return;
  }
  snapshotAt = performance.now();

  const s = task.step;
  if (!prev || prev.step.text !== s.text || prev.sessionId !== task.sessionId) {
    replay(step, "swap");
    nudgeAsked = false;
  }
  if (prev?.sessionId !== task.sessionId) lastPeek = null;
  wrap.classList.toggle("paused", task.paused);
  const pauseHint = task.paused ? "Continuar o timer" : "Pausar o timer";
  pauseBtn.dataset.hint = pauseHint;
  pauseBtn.setAttribute("aria-label", pauseHint);
  updateShrink();
  stepText.textContent = s.text;
  step.title = s.text;
  kind.textContent = KIND_LABEL[s.kind];
  kind.hidden = s.kind === "normal";
  const sub =
    s.kind === "watch" && task.tutorial ? `a partir de ${task.tutorial.position}` :
    s.kind === "refs" ? `${s.refsCount} de ${REFS_GOAL}` : "";
  stepSub.textContent = sub;
  stepSub.hidden = !sub;
  $("plusBtn").hidden = s.kind !== "refs";
  $("fullBtn").hidden = s.kind === "refs";
  tick();
  reportShape();
}

function tick() {
  if (!task) return;
  const elapsed = task.elapsedMs + (task.paused ? 0 : performance.now() - snapshotAt);
  const clock = formatClock(elapsed);
  const pct = Math.min(100, (elapsed / FIVE_MIN) * 100);
  time.textContent = clock;
  fill.style.width = `${pct}%`;
  dialTime.textContent = clock;
  dialTime.classList.toggle("long", clock.length > 5);
  dialFill.style.strokeDashoffset = `${100 - pct}`;
  const flow = elapsed >= FIVE_MIN;
  bar.classList.toggle("flow", flow);
  dial.classList.toggle("flow", flow);
  mascot.setBase(task.paused ? "idle" : flow ? "flow" : "work");
  if (flow && !flowLogged.has(task.sessionId)) {
    flowLogged.add(task.sessionId);
    invoke("log_event", { kind: "marca_5min", detail: task.title });
  }
  const interval = Math.floor(elapsed / (peekMin * 60_000));
  if (lastPeek !== null && interval > lastPeek && mode === "dot") peek();
  lastPeek = interval;
  const s = task.step;
  if (s.kind === "refs" && !s.nudged && !nudgeAsked && elapsed - s.refsStartedMs >= REFS_NUDGE_MS) {
    nudgeAsked = true;
    invoke("refs_nudge");
  }
}

// Once a second is all a clock that shows seconds needs.
setInterval(tick, 1000);

/** Tells the core which parts of this window are painted; clicks elsewhere go to the app below. */
function reportShape() {
  requestAnimationFrame(() => {
    const painted = compact ? [dial, mascotEl, ...(open ? [bar] : [])] : [bar, mascotEl];
    const rects = painted.filter((el) => !el.hidden).map((el) => {
      const r = el.getBoundingClientRect();
      return [r.left, r.top, r.width, r.height];
    });
    invoke("set_bar_solid", { rects });
  });
}
new ResizeObserver(reportShape).observe(bar);
// Growing or shrinking is a transform, which ResizeObserver doesn't see.
dialWrap.addEventListener("transitionend", (e) => e.target === dialWrap && reportShape());

/** Minimized shows as a dot, except while hovered, peeking or paused (a paused timer shouldn't hide). */
function updateShrink() {
  dialWrap.classList.toggle("min", mode === "dot" && !open && !peeking && !task?.paused);
  reportShape();
}

/** Shows the time for a few seconds, then goes back to a dot. */
function peek() {
  peeking = true;
  updateShrink();
  clearTimeout(peekTimer);
  peekTimer = window.setTimeout(() => {
    peeking = false;
    updateShrink();
  }, PEEK_MS);
}

/** Opens right away, closes after a beat so the pointer can cross the gap to the buttons. */
function setOpen(on: boolean) {
  clearTimeout(closeTimer);
  if (on === open) return;
  if (on) {
    open = true;
    wrap.classList.add("open");
    updateShrink();
  } else {
    closeTimer = window.setTimeout(() => {
      open = false;
      wrap.classList.remove("open");
      setHover(null);
      updateShrink();
    }, 400);
  }
}

function applySettings(s: Settings) {
  mascot.setEnabled(s.mascot);
  mode = s.timer;
  if (s.peekMin !== peekMin) {
    peekMin = s.peekMin;
    lastPeek = null;
  }
  if ((mode !== "bar") !== compact) {
    compact = mode !== "bar";
    clearTimeout(closeTimer);
    open = false;
    wrap.classList.remove("open");
    wrap.classList.toggle("compact", compact);
    dialWrap.hidden = !compact;
    // The mascot sits on whichever is showing: the bar's top edge or the circle's.
    if (compact) dialWrap.append(mascotEl);
    else wrap.prepend(mascotEl);
  }
  updateShrink();
}
listen<Settings>("settings", (e) => applySettings(e.payload));
invoke<Settings>("get_settings").then(applySettings);

// The box opening is a moment for the mascot too.
const BOX_MOMENT: Record<string, Moment> = { lost: "look", full: "look", refsDone: "cheer" };
listen<{ mode: string }>("box-mode", (e) => {
  const moment = BOX_MOMENT[e.payload.mode];
  if (moment) mascot.play(moment);
});
// Breathes along with the circle in the box.
listen<{ phase: "inhale" | "exhale" | "end" }>("breath", (e) => {
  if (e.payload.phase === "end") mascot.release();
  else mascot.hold(e.payload.phase);
});

listen<StateView>("state", (e) => apply(e.payload));
// ⌥ Space mid-task: also the way to see the time on demand while minimized.
listen("nudge", () => {
  if (mode === "dot") peek();
  replay(compact ? dial : bar, "nudge");
});
invoke<StateView>("get_state").then(apply);

// ---------- buttons ----------

// Native tooltips don't show on a window that never takes focus, so the step area
// doubles as the label of whatever button is under the pointer.
const MODS = navigator.userAgent.includes("Mac") ? "⌥" : "Ctrl Alt";
const hint = $("hint");
let hovered: HTMLElement | null = null;

function setHover(b: HTMLElement | null) {
  if (b === hovered) return;
  hovered?.classList.remove("hover", "pressed");
  hovered = b;
  if (!b) {
    hint.classList.remove("on");
    return;
  }
  b.classList.add("hover");
  $("hintText").textContent = b.dataset.hint ?? "";
  $("hintKey").textContent = b.dataset.key ? `${MODS} ${b.dataset.key}` : "";
  $("hintKey").hidden = !b.dataset.key;
  hint.classList.add("on");
}

// The core reports the cursor even while the bar is inactive (see start_hover_tracking).
listen<[number, number]>("pointer", ({ payload: [x, y] }) => {
  const el = x < 0 ? null : document.elementFromPoint(x, y);
  setHover(el?.closest<HTMLElement>(".ibtn:not([hidden])") ?? null);
  if (compact) setOpen(!!el?.closest("#dialWrap, #bar"));
});

for (const b of document.querySelectorAll<HTMLButtonElement>(".ibtn")) {
  b.addEventListener("pointerenter", () => setHover(b));
  b.addEventListener("pointerleave", () => setHover(null));
  b.addEventListener("pointerdown", () => b.classList.add("pressed"));
  b.addEventListener("pointerup", () => b.classList.remove("pressed"));
}

function press(id: string, cls: string, run: () => unknown) {
  const b = $(id);
  b.addEventListener("click", () => {
    replay(b, cls);
    setHover(null);
    run();
  });
}

press("doneBtn", "ok", () => {
  mascot.play("cheer");
  invoke("complete_step");
});
press("pauseBtn", "pop", () => invoke("toggle_pause"));
press("plusBtn", "ok", () => {
  replay(stepSub, "bump");
  invoke("add_ref");
});
press("lostBtn", "pop", () => invoke("show_box", { mode: "lost" }));
press("fullBtn", "pop", () => invoke("show_box", { mode: "full" }));
