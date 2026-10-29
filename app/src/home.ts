// The overview: a look at the day on request. Starting anything from here goes through the box.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";
import { escapeHtml as esc, formatClock, formatHuman, type Kind, type StateView, type Thought } from "./shared";

type DoneStep = { text: string; kind: string; marker: string | null; ms: number };
type DoneItem = { title: string; tutorial: boolean; startedAt: number; endedAt: number; elapsedMs: number; steps: DoneStep[] };
type PlanItem = { id: number; text: string; createdAt: number };
type Overview = {
  day: { done: DoneItem[]; focusMs: number; stepsDone: number };
  plan: PlanItem[];
  thoughts: Thought[];
};

const $ = (id: string) => document.getElementById(id)!;
const DAY = 24 * 3600 * 1000;
const MAC = navigator.userAgent.includes("Mac");
const OPEN_KEYS = MAC ? "⌥ Espaço" : "Ctrl Espaço";
$("soundKeys").textContent = MAC ? "⌥ S" : "Ctrl Alt S";
const KIND_LABEL: Record<Kind, string> = { normal: "", watch: "tutorial · assistir", practice: "tutorial · praticar", refs: "referências" };

let state: StateView | null = null;
let data: Overview | null = null;
/** 0 = today, -1 = yesterday… Only the "Feito" column moves. */
let dayOffset = 0;
let snapshotAt = 0;
/** Done items stay open across refreshes. */
const openDone = new Set<string>();

const startOfDay = (offset: number) => {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  d.setDate(d.getDate() + offset);
  return d.getTime();
};
const clock = (ms: number) => new Date(ms).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
const short = (ms: number) => (ms < 60000 ? "<1 min" : formatHuman(ms));
const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

function dayName(offset: number) {
  if (offset === 0) return "hoje";
  if (offset === -1) return "ontem";
  return new Date(startOfDay(offset)).toLocaleDateString("pt-BR", { weekday: "long", day: "numeric", month: "short" });
}

const liveElapsed = () => (state?.task ? state.task.elapsedMs + (performance.now() - snapshotAt) : 0);

// ---------- panels ----------

function renderNow() {
  const t = state?.task;
  let html: string;
  if (t) {
    const kind = KIND_LABEL[t.step.kind];
    html = `
      <p class="eyebrow">Agora${kind ? ` · ${kind}` : ""}</p>
      <p class="now-title">${esc(t.title)}</p>
      <p class="now-step"><span>Passo:</span> ${esc(t.step.text)}</p>
      <p class="now-time" id="nowTime">${formatClock(liveElapsed())}</p>`;
  } else {
    html = `
      <p class="eyebrow">Agora</p>
      <p class="now-title muted">Nada em andamento.</p>
      <div class="row"><button class="btn primary" type="button" data-act="newTask">Começar algo <span class="key">${OPEN_KEYS}</span></button></div>`;
  }
  if (state?.parkedTitle) {
    const where = state.parkedPosition ? ` · parou em ${esc(state.parkedPosition)}` : "";
    html += `
      <div class="parked">
        <p class="sub">Guardada${where}</p>
        <p class="parked-title">${esc(state.parkedTitle)}</p>
        <div class="row">
          ${t ? "" : `<button class="btn" type="button" data-act="resume">Retomar</button>`}
          <button class="btn quiet" type="button" data-act="dropParked">Soltar</button>
        </div>
      </div>`;
  }
  $("now").innerHTML = html;
}

function renderDone() {
  if (!data) return;
  const { done, focusMs, stepsDone } = data.day;
  const today = dayOffset === 0;
  // Today's focus time includes the task still running.
  const focus = focusMs + (today ? liveElapsed() : 0);
  $("doneTitle").textContent = `Feito ${dayName(dayOffset)}`;
  ($("nextDay") as HTMLButtonElement).disabled = today;
  $("stats").innerHTML = `
    <div class="stat"><b>${done.length}</b><span>${done.length === 1 ? "tarefa" : "tarefas"}</span></div>
    <div class="stat"><b id="focusStat">${focus < 60000 ? "0 min" : formatHuman(focus)}</b><span>de foco</span></div>
    <div class="stat"><b>${stepsDone}</b><span>${stepsDone === 1 ? "passo" : "passos"}</span></div>`;
  if (!done.length) {
    $("doneList").innerHTML = `<li class="empty">${today ? "Nada concluído ainda hoje. Um passo de cada vez." : "Nada concluído neste dia."}</li>`;
    return;
  }
  // Newest first: what you just did is what you want to see.
  $("doneList").innerHTML = [...done]
    .reverse()
    .map((d) => {
      const key = `${d.endedAt}`;
      const steps = d.steps
        .map(
          (s) => `<li><span class="step-name">${esc(s.text)}${s.marker && s.kind === "referencias" ? ` <i>(${esc(s.marker)})</i>` : ""}</span>
            <span class="dur">${short(s.ms)}</span></li>`,
        )
        .join("");
      return `<li>
        <details data-key="${key}" ${openDone.has(key) ? "open" : ""}>
          <summary>
            <span class="at">${clock(d.endedAt)}</span>
            <span class="done-title">${esc(d.title)}${d.tutorial ? ` <span class="tag">tutorial</span>` : ""}</span>
            <span class="dur">${short(d.elapsedMs)}</span>
          </summary>
          <p class="sub">${clock(d.startedAt)} → ${clock(d.endedAt)} · ${plural(d.steps.length, "passo", "passos")}</p>
          ${steps ? `<ul class="steps">${steps}</ul>` : ""}
        </details>
      </li>`;
    })
    .join("");
}

function renderPlan() {
  if (!data) return;
  const today = startOfDay(0);
  const items = data.plan;
  $("planCount").textContent = items.length ? String(items.length) : "";
  $("planList").innerHTML = items.length
    ? items
        .map((p) => {
          const since = p.createdAt < today ? `<span class="sub">desde ${p.createdAt < today - DAY ? new Date(p.createdAt).toLocaleDateString("pt-BR", { day: "numeric", month: "short" }) : "ontem"}</span>` : "";
          return `<li>
            <div class="item-text"><span>${esc(p.text)}</span>${since}</div>
            <div class="item-acts">
              <button class="btn sm" type="button" data-act="startPlan" data-id="${p.id}">Começar</button>
              <button class="btn quiet sm" type="button" data-act="removePlan" data-id="${p.id}" aria-label="Tirar da lista">✕</button>
            </div>
          </li>`;
        })
        .join("")
    : `<li class="empty">Nada anotado. Tudo bem: dá pra seguir uma tarefa de cada vez.</li>`;
}

function renderDrawer() {
  if (!data) return;
  const n = data.thoughts.length;
  $("drawerPanel").hidden = n === 0;
  $("drawerTitle").innerHTML = `Gaveta <span class="count">${plural(n, "pensamento", "pensamentos")}</span>`;
  $("drawerList").innerHTML = data.thoughts
    .map(
      (t) => `<li>
        <div class="item-text"><span>${esc(t.text)}</span></div>
        <div class="item-acts">
          <button class="btn sm" type="button" data-act="toPlan" data-id="${t.id}">Pra hoje</button>
          <button class="btn quiet sm" type="button" data-act="delThought" data-id="${t.id}" aria-label="Apagar">✕</button>
        </div>
      </li>`,
    )
    .join("");
}

function renderAll() {
  const title = new Date().toLocaleDateString("pt-BR", { weekday: "long", day: "numeric", month: "long" });
  $("today").textContent = title.charAt(0).toUpperCase() + title.slice(1);
  renderNow();
  renderDone();
  renderPlan();
  renderDrawer();
}

async function load() {
  const from = startOfDay(dayOffset);
  const [s, o] = await Promise.all([
    invoke<StateView>("get_state"),
    invoke<Overview>("overview", { from, to: from + DAY }),
  ]);
  state = s;
  snapshotAt = performance.now();
  data = o;
  renderAll();
}

// Only the numbers that move; everything else waits for a state change.
setInterval(() => {
  if (!state?.task) return;
  const el = document.getElementById("nowTime");
  if (el) el.textContent = formatClock(liveElapsed());
  const focus = document.getElementById("focusStat");
  if (focus && data && dayOffset === 0) {
    const ms = data.day.focusMs + liveElapsed();
    focus.textContent = ms < 60000 ? "0 min" : formatHuman(ms);
  }
}, 1000);

// ---------- actions ----------

const idOf = (el: HTMLElement) => Number(el.dataset.id);
const actions: Record<string, (el: HTMLElement) => unknown> = {
  close: () => invoke("hide_home_window"),
  sounds: () => invoke("open_sounds"),
  newTask: () => invoke("new_task"),
  resume: () => invoke("resume_parked"),
  dropParked: () => invoke("drop_parked"),
  startPlan: (el) => invoke("start_plan", { id: idOf(el) }),
  removePlan: (el) => invoke("remove_plan", { id: idOf(el) }),
  toPlan: (el) => invoke("thought_to_plan", { id: idOf(el) }),
  delThought: (el) => invoke("resolve_thought", { id: idOf(el), outcome: "apagado" }),
  prevDay: () => {
    dayOffset--;
    load();
  },
  nextDay: () => {
    dayOffset = Math.min(0, dayOffset + 1);
    load();
  },
};

document.addEventListener("click", (e) => {
  const el = (e.target as HTMLElement).closest<HTMLElement>("[data-act]");
  if (el) actions[el.dataset.act!]?.(el);
});

document.addEventListener(
  "toggle",
  (e) => {
    const d = e.target as HTMLDetailsElement;
    if (!d.dataset?.key) return;
    if (d.open) openDone.add(d.dataset.key);
    else openDone.delete(d.dataset.key);
  },
  true,
);

const planInput = $("planInput") as HTMLInputElement;
planInput.addEventListener("keydown", async (e) => {
  if (e.key !== "Enter") return;
  e.preventDefault();
  const text = planInput.value.trim();
  if (!text) return;
  planInput.value = "";
  await invoke("add_plan", { text });
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    e.preventDefault();
    if (document.activeElement === planInput && planInput.value) planInput.value = "";
    else invoke("hide_home_window");
  }
});

// Each time it comes back into view it starts from today.
window.addEventListener("focus", () => {
  dayOffset = 0;
  load();
});
listen("state", () => load());
load();
