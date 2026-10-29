// The entry box: the only window that takes the keyboard, so every conversation lives here.
import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { Mascot } from "./mascot";
import "./styles.css";
import {
  REFS_GOAL,
  escapeHtml as esc,
  formatHuman,
  replay,
  type Settings,
  type StateView,
  type TaskView,
  type Thought,
} from "./shared";

type Stage =
  | "intent" | "tutOffer" | "tutUntil" | "firstStep"
  | "next" | "practice" | "watchUntil"
  | "lost" | "lost2" | "smaller" | "park" | "where" | "parkConfirm"
  | "dump" | "breathe" | "back" | "pick"
  | "done" | "drawer"
  | "refsDone" | "refsNudge";

const card = document.getElementById("card")!;
const TUTORIAL_RE = /\b(aula|tutorial|curso|v[ií]deo)\b|https?:\/\//i;
const PRACTICE_DEFAULT = "Reproduzir no AE o que viu";
const DRAWER_DAYS = 7;
const DAY = 24 * 3600 * 1000;

let state: StateView = { task: null, parkedTitle: null, parkedTutorial: false, parkedPosition: null, drawerCount: 0 };
let stage: Stage = "intent";
const ctx = {
  title: "",
  /** A thought chosen as "more urgent"; it becomes the next task after park/drop. */
  pending: null as string | null,
  /** "Pra hoje" item being started; it leaves the list once the task starts. */
  planId: null as number | null,
  /** Video minute given when parking a tutorial. */
  position: null as string | null,
  dump: [] as Thought[],
  drawer: [] as Thought[],
  doneMsg: "",
  resumeMsg: "",
};
let breathToken = 0;
let mascotOn = true;
let hideTimer: number | undefined;

const stepText = () => state.task?.step.text ?? "";
const taskTitle = () => state.task?.title ?? ctx.title;
const videoPosition = () => state.task?.tutorial?.position ?? "0:00";
const log = (kind: string, detail?: string) => invoke("log_event", { kind, detail: detail ?? null });
const hide = () => {
  if (stage === "breathe") breath("end");
  return invoke("hide_box");
};
const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;

/** Tutorials get offered the watch/practice split; everything else asks for a first step. */
const startStage = (title: string): Stage => (TUTORIAL_RE.test(title) ? "tutOffer" : "firstStep");

function parseMinute(v: string): string | null {
  const m = v.trim().match(/^(\d{1,3})(?:[:.,]([0-5]\d))?$/);
  return m ? `${Number(m[1])}:${m[2] ?? "00"}` : null;
}

function when(createdAt: number): string {
  const days = Math.floor((Date.now() - createdAt) / DAY);
  if (days < 1) {
    const t = new Date(createdAt).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
    return `hoje às ${t}`;
  }
  return days === 1 ? "ontem" : `há ${days} dias`;
}

const daysLeft = (createdAt: number) => Math.max(1, DRAWER_DAYS - Math.floor((Date.now() - createdAt) / DAY));

// ---------- templates ----------

// Buttons get a number key (1–9) so stages without a text field work from the keyboard.
let keyIndex = 0;
function btn(act: string, label: string, style = "", extra = "") {
  keyIndex++;
  const key = keyIndex <= 9 ? `<span class="key">${keyIndex}</span>` : "";
  return `<button class="btn ${style}" type="button" data-act="${act}" ${extra}>${key}${label}</button>`;
}

const field = (id: string, submit: string, placeholder: string, extra = "") =>
  `<input class="field" id="${id}" data-submit="${submit}" autocomplete="off" spellcheck="false" placeholder="${esc(placeholder)}" ${extra} />`;

const taskLine = (title = taskTitle()) =>
  title ? `<p class="entry-task">Tarefa: <b>${esc(title)}</b></p>` : "";

const VIEWS: Record<Stage, () => string> = {
  intent: () => `
    <h1><label for="intent">O que precisa ser feito agora?</label></h1>
    ${field("intent", "intent", "Ex.: fazer o relatório")}
    <p class="hint">Enter para continuar · Esc para fechar</p>`,

  tutOffer: () => `
    ${taskLine()}
    <h1>Parece um tutorial. Dividir em assistir e praticar?</h1>
    <p class="sub">Você assiste um trecho, pratica no After Effects e volta para o próximo trecho.</p>
    <div class="row">${btn("tutYes", "Sim, dividir", "primary")}${btn("tutNo", "Não, passos normais")}</div>`,

  tutUntil: () => `
    ${taskLine()}
    <h1><label for="until">Até que minuto vai assistir agora?</label></h1>
    ${field("until", "tutStart", "Ex.: 6:30", 'inputmode="numeric"')}
    <p class="hint">Um trecho curto, até o próximo conceito. Enter começa.</p>`,

  firstStep: () => `
    ${taskLine()}
    <h1><label for="s1">Qual o primeiro passo minúsculo?</label></h1>
    ${field("s1", "start", "Ex.: abrir a planilha")}
    <button class="linkbtn" type="button" data-act="more">+ adicionar mais passos</button>
    <div class="stack" id="more" hidden>
      <input class="field sm" id="s2" data-submit="start" autocomplete="off" spellcheck="false" placeholder="Passo 2 (opcional)" />
      <input class="field sm" id="s3" data-submit="start" autocomplete="off" spellcheck="false" placeholder="Passo 3 (opcional)" />
    </div>
    <p class="hint">Algo que dá para começar em menos de 2 minutos. Enter começa.</p>`,

  next: () => `
    ${taskLine()}
    <h1><label for="next">E agora?</label></h1>
    ${field("next", "next", "Próximo passo minúsculo")}
    <div class="row">${btn("finish", "Terminei a tarefa")}</div>
    <p class="hint">Enter começa o passo · Esc fecha</p>`,

  practice: () => `
    ${taskLine()}
    <h1><label for="practice">Agora pratique o que viu.</label></h1>
    ${field("practice", "practice", "O que vai reproduzir", `value="${PRACTICE_DEFAULT}" data-select="1"`)}
    <div class="row">${btn("skipPractice", "Pular, assistir mais", "quiet")}</div>
    <p class="hint">Enter começa · edite se quiser algo mais específico</p>`,

  watchUntil: () => `
    ${taskLine()}
    ${ctx.resumeMsg ? `<p class="sub">${esc(ctx.resumeMsg)}</p>` : ""}
    <h1><label for="until">Até que minuto vai assistir agora?</label></h1>
    ${field("until", "watch", `Depois de ${videoPosition()}. Ex.: 12:00`, 'inputmode="numeric"')}
    <div class="row">${btn("finish", "Terminei o tutorial")}</div>`,

  lost: () => `
    <h1>Ainda está em <b>“${esc(stepText())}”</b>?</h1>
    <div class="row">${btn("yes", "Sim", "primary")}${btn("no", "Não")}</div>`,

  lost2: () => `
    <h1>Tudo bem. A tarefa travou ou você quer mudar de assunto?</h1>
    <div class="row">${btn("stuck", "Travou")}${btn("switch", "Mudar de assunto")}${btn("hide", "Voltar ao passo", "quiet")}</div>`,

  smaller: () => `
    <h1><label for="smaller">Qual um passo ainda menor?</label></h1>
    <p class="sub">Ex.: em vez de “escrever a introdução”, “escrever a primeira frase”.</p>
    ${field("smaller", "smaller", "Passo menor")}`,

  park: () => `
    <h1>Guardar <b>“${esc(taskTitle())}”</b> para depois ou soltar?</h1>
    <p class="sub">${ctx.pending ? `Depois você começa “${esc(ctx.pending)}”. ` : ""}Se guardar, ela volta quando você terminar a próxima tarefa.</p>
    <div class="row">${btn("keep", "Guardar", "primary")}${btn("drop", "Soltar")}</div>`,

  where: () => `
    <h1><label for="where">Parou em que minuto?</label></h1>
    <p class="sub">Para você voltar exatamente daqui.</p>
    ${field("where", "where", `Ex.: ${videoPosition()}`, 'inputmode="numeric"')}`,

  parkConfirm: () => `
    <h1>Isso vai soltar <b>“${esc(state.parkedTitle ?? "")}”</b>, que já estava guardada. Ok?</h1>
    <div class="row">${btn("park", "Ok, guardar a nova", "primary")}${btn("toPark", "Voltar")}</div>`,

  dump: () => `
    <h1><label for="dump">O que está passando na sua cabeça?</label></h1>
    ${field("dump", "dump", "Um pensamento, depois Enter")}
    <div class="row"><span class="count" id="dumpCount">${dumpCountText()}</span>${btn("dumpDone", "Pronto", "primary")}</div>`,

  breathe: () => `
    <div class="breath">
      <div class="breath-ring"><div class="circle" id="circle"></div></div>
      <span class="breath-label" id="breathLabel">Inspira</span>
      <span class="sub" id="breathCount">1 de 3</span>
    </div>
    <div class="row">${btn("skipBreath", "Pular", "quiet")}</div>`,

  back: () => `
    <p class="sub">${ctx.dump.length ? "Tudo anotado. " : ""}Agora, só isto:</p>
    <p class="big">${esc(stepText())}</p>
    <div class="row">${btn("backToStep", "Voltar a isso", "primary")}${ctx.dump.length ? btn("urgent", "Algo que anotei é mais urgente") : ""}</div>`,

  pick: () => `
    <h1>Qual delas?</h1>
    <div class="stack">${ctx.dump.map((d, i) => btn("pick", esc(d.text), "pick", `data-i="${i}"`)).join("")}</div>
    <div class="row">${btn("toBack", "Nenhuma, voltar", "quiet")}</div>`,

  done: () => {
    // The bar is gone by now, so the mascot celebrates here.
    const head = `<div class="done-head"><div id="soloMascot"></div><h1>${esc(ctx.doneMsg)}</h1></div>`;
    if (state.parkedTitle) {
      return `${head}
        <p class="sub">Você tinha guardado “${esc(state.parkedTitle)}”.</p>
        <div class="row">${btn("resume", "Retomar", "primary")}${btn("dropParked", "Soltar")}</div>`;
    }
    if (state.drawerCount) {
      return `${head}
        <p class="sub">Tem ${plural(state.drawerCount, "coisa", "coisas")} na gaveta. Ver agora?</p>
        <div class="row">${btn("drawer", "Ver", "primary")}${btn("hide", "Depois")}</div>`;
    }
    return head;
  },

  drawer: () => {
    if (!ctx.drawer.length) {
      return `<h1>A gaveta está vazia.</h1><div class="row">${btn("hide", "Fechar", "primary")}</div>`;
    }
    const items = ctx.drawer
      .map(
        (d) => `<li><span>${esc(d.text)}</span>
          <span class="sub">anotado ${when(d.createdAt)} · some em ${plural(daysLeft(d.createdAt), "dia", "dias")}</span>
          <div class="row">${btn("toTask", "Virar tarefa", "", `data-id="${d.id}"`)}${btn("del", "Apagar", "quiet", `data-id="${d.id}"`)}</div></li>`,
      )
      .join("");
    return `<h1>Gaveta</h1><ul class="items">${items}</ul><div class="row">${btn("hide", "Fechar")}</div>`;
  },

  refsDone: () => `
    <h1>Pronto: ${state.task?.step.refsCount ?? REFS_GOAL} referências.</h1>
    <p class="sub">Hora de usar o que você juntou.</p>
    <div class="row">${btn("refsEnough", "Seguir", "primary")}</div>`,

  refsNudge: () => `
    <h1>${state.task?.step.refsCount ?? 0} de ${REFS_GOAL}. Já dá?</h1>
    <p class="sub">Você está buscando referências há 15 minutos.</p>
    <div class="row">${btn("refsEnough", "Já tenho o suficiente", "primary")}${btn("hide", "Continuar")}</div>`,
};

function dumpCountText() {
  return ctx.dump.length ? `${plural(ctx.dump.length, "guardado", "guardados")} na gaveta` : "Enter guarda cada um";
}

// ---------- rendering ----------

/** Lets the mascot on the bar breathe along. */
const breath = (phase: "inhale" | "exhale" | "end") => emitTo("bar", "breath", { phase });

function go(next: Stage) {
  if (stage === "breathe" && next !== "breathe") breath("end");
  stage = next;
  clearTimeout(hideTimer);
  breathToken++;
  keyIndex = 0;
  card.innerHTML = VIEWS[next]();
  const input = card.querySelector<HTMLInputElement>("input:not([hidden] input)");
  if (input) {
    // Number keys would type into the field, so don't advertise them.
    card.querySelectorAll(".key").forEach((k) => k.remove());
    input.focus();
    if (input.dataset.select) input.select();
  } else {
    card.querySelector<HTMLElement>(".btn.primary, .btn")?.focus();
  }
  if (next === "breathe") breathe();
  const solo = document.getElementById("soloMascot");
  if (solo) {
    if (mascotOn) new Mascot(solo, false).play("finish");
    else solo.remove();
  }
}

function shake(input: HTMLInputElement) {
  replay(input, "shake");
  input.focus();
}

function hideSoon() {
  hideTimer = window.setTimeout(hide, 1800);
}

/** 3 cycles of 4 s in, 6 s out. A longer exhale is what slows things down. */
function breathe() {
  const token = breathToken;
  const alive = () => token === breathToken && stage === "breathe";
  const circle = document.getElementById("circle")!;
  const label = document.getElementById("breathLabel")!;
  const count = document.getElementById("breathCount")!;
  let cycle = 0;
  const inhale = () => {
    if (!alive()) return;
    cycle++;
    label.textContent = "Inspira";
    count.textContent = `${cycle} de 3`;
    breath("inhale");
    circle.style.transitionDuration = "4s";
    circle.style.transform = "scale(1.8)";
    setTimeout(exhale, 4000);
  };
  const exhale = () => {
    if (!alive()) return;
    label.textContent = "Solta devagar";
    breath("exhale");
    circle.style.transitionDuration = "6s";
    circle.style.transform = "scale(1)";
    setTimeout(() => {
      if (!alive()) return;
      if (cycle < 3) return inhale();
      log("respiracao_completa");
      go("back");
    }, 6000);
  };
  requestAnimationFrame(() => requestAnimationFrame(inhale));
}

function flyAway(text: string, input: HTMLInputElement) {
  const fly = document.createElement("span");
  fly.className = "fly";
  fly.textContent = text;
  fly.style.top = `${input.offsetTop + 8}px`;
  card.appendChild(fly);
  fly.addEventListener("animationend", () => fly.remove());
  setTimeout(() => fly.remove(), 1000);
}

// ---------- flow ----------

async function park() {
  await invoke("park_task", { position: ctx.position });
  ctx.position = null;
  afterSwitch();
}

/** After parking or dropping: start the chosen thought, or a fresh intention. */
function afterSwitch() {
  state = { ...state, task: null };
  if (ctx.pending) {
    ctx.title = ctx.pending;
    ctx.pending = null;
    go(startStage(ctx.title));
  } else {
    ctx.title = "";
    ctx.planId = null;
    go("intent");
  }
}

async function showDone() {
  state = await invoke<StateView>("get_state");
  go("done");
  if (!state.parkedTitle && !state.drawerCount) hideSoon();
}

const actions: Record<string, (el: HTMLElement) => unknown> = {
  hide,
  more: (el) => {
    el.hidden = true;
    document.getElementById("more")!.hidden = false;
    document.getElementById("s2")!.focus();
  },
  tutYes: () => go("tutUntil"),
  tutNo: () => go("firstStep"),
  finish: async () => {
    const task = await invoke<TaskView | null>("finish_task");
    if (!task) return hide();
    ctx.doneMsg = `Feito. ${formatHuman(task.elapsedMs)} em “${task.title}”.`;
    await showDone();
  },
  skipPractice: () => {
    log("pratica_pulada");
    ctx.resumeMsg = "";
    go("watchUntil");
  },
  yes: () => {
    log("foco_confirmado", stepText());
    hide();
  },
  no: () => go("lost2"),
  stuck: () => {
    log("travou", stepText());
    go("smaller");
  },
  switch: () => {
    log("mudou_de_assunto", taskTitle());
    go("park");
  },
  toPark: () => go("park"),
  keep: () => {
    if (state.task?.tutorial) go("where");
    else if (state.parkedTitle) go("parkConfirm");
    else park();
  },
  park,
  drop: async () => {
    await invoke("drop_task");
    afterSwitch();
  },
  dumpDone: () => go("breathe"),
  skipBreath: () => {
    log("respiracao_pulada");
    go("back");
  },
  backToStep: () => {
    log("voltou_ao_passo", stepText());
    hide();
  },
  urgent: () => go("pick"),
  toBack: () => go("back"),
  pick: async (el) => {
    const thought = ctx.dump[Number(el.dataset.i)];
    await invoke("resolve_thought", { id: thought.id, outcome: "virou_tarefa" });
    ctx.pending = thought.text;
    ctx.planId = null;
    go("park");
  },
  resume: async () => {
    const task = await invoke<TaskView | null>("resume_parked");
    if (!task?.tutorial) return hide();
    state = await invoke<StateView>("get_state");
    ctx.resumeMsg = `Você parou em ${task.tutorial.position} de “${task.title}”.`;
    go("watchUntil");
  },
  dropParked: async () => {
    await invoke("drop_parked");
    await showDone();
  },
  drawer: async () => {
    ctx.drawer = await invoke<Thought[]>("list_thoughts");
    go("drawer");
  },
  toTask: async (el) => {
    const text = await invoke<string | null>("resolve_thought", { id: Number(el.dataset.id), outcome: "virou_tarefa" });
    if (!text) return;
    ctx.title = text;
    go(startStage(text));
  },
  del: async (el) => {
    await invoke("resolve_thought", { id: Number(el.dataset.id), outcome: "apagado" });
    ctx.drawer = await invoke<Thought[]>("list_thoughts");
    go("drawer");
  },
  refsEnough: () => {
    log("refs_suficiente", `${state.task?.step.refsCount ?? 0} referências`);
    invoke("complete_step");
  },
};

const setStep = (text: string | null, kind: string, until: string | null = null, reason: string | null = null) =>
  invoke("set_step", { text, kind, until, reason });

const submits: Record<string, (value: string, input: HTMLInputElement) => unknown> = {
  intent: (v) => {
    ctx.title = v;
    go(startStage(v));
  },
  tutStart: (v, input) => {
    const until = parseMinute(v);
    if (!until) return shake(input);
    return invoke("start_task", { title: ctx.title, steps: [], watchUntil: until, planId: ctx.planId });
  },
  start: () => {
    const steps = ["s1", "s2", "s3"].map((id) => (document.getElementById(id) as HTMLInputElement).value);
    return invoke("start_task", { title: ctx.title, steps, watchUntil: null, planId: ctx.planId });
  },
  next: (v) => setStep(v, "normal"),
  practice: (v) => setStep(v, "practice"),
  watch: (v, input) => {
    const until = parseMinute(v);
    if (!until) return shake(input);
    ctx.resumeMsg = "";
    return setStep(null, "watch", until);
  },
  smaller: (v) => setStep(v, "normal", null, "passo_reduzido"),
  where: (v, input) => {
    const position = parseMinute(v);
    if (!position) return shake(input);
    ctx.position = position;
    if (state.parkedTitle) go("parkConfirm");
    else return park();
  },
  dump: async (v, input) => {
    const thought = await invoke<Thought>("add_thought", { text: v });
    ctx.dump.push(thought);
    flyAway(v, input);
    input.value = "";
    const count = document.getElementById("dumpCount")!;
    count.textContent = dumpCountText();
    replay(count, "bump");
  },
};

// ---------- events ----------

card.addEventListener("click", (e) => {
  const el = (e.target as HTMLElement).closest<HTMLElement>("[data-act]");
  if (el) actions[el.dataset.act!]?.(el);
});

card.addEventListener("keydown", async (e) => {
  if (e.key !== "Enter") return;
  const input = (e.target as HTMLElement).closest<HTMLInputElement>("input[data-submit]");
  if (!input) return;
  e.preventDefault();
  // In the first-step form the required field is always s1, whichever field had Enter.
  const required = input.dataset.submit === "start" ? (document.getElementById("s1") as HTMLInputElement) : input;
  if (!required.value.trim()) return shake(required);
  try {
    await submits[input.dataset.submit!](input.value.trim(), input);
  } catch {
    shake(required);
  }
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    e.preventDefault();
    hide();
    return;
  }
  if (/^[1-9]$/.test(e.key) && !(e.target instanceof HTMLInputElement)) {
    const key = [...card.querySelectorAll<HTMLElement>(".key")].find((k) => k.textContent === e.key);
    const button = key?.closest<HTMLElement>("[data-act]");
    if (!button || button.classList.contains("pressed")) return;
    // Show the press before the stage changes, so the key feels like a click.
    button.classList.add("pressed");
    setTimeout(() => {
      button.classList.remove("pressed");
      button.click();
    }, 120);
  }
});

// The window is reused: put the cursor back in the right place whenever it reappears.
window.addEventListener("focus", () => {
  const input = card.querySelector<HTMLInputElement>("input");
  (input ?? card.querySelector<HTMLElement>(".btn.primary, .btn"))?.focus();
});

type BoxMode = { mode: string; state: StateView; title: string | null; planId: number | null };
listen<BoxMode>("box-mode", (e) => {
  state = e.payload.state;
  ctx.pending = null;
  ctx.position = null;
  ctx.resumeMsg = "";
  ctx.planId = e.payload.planId;
  const mode = e.payload.mode;
  if (mode === "plan") {
    ctx.title = e.payload.title ?? "";
    go(startStage(ctx.title));
  } else if (mode === "planSwitch") {
    // Something is running: park or drop it, then start the list item.
    ctx.title = state.task?.title ?? "";
    ctx.pending = e.payload.title;
    go("park");
  } else if (mode === "intent") {
    ctx.title = "";
    go("intent");
  } else if (mode === "full") {
    ctx.dump = [];
    go("dump");
  } else {
    ctx.title = state.task?.title ?? "";
    go(mode as Stage);
  }
});

listen<StateView>("state", (e) => {
  state = e.payload;
});

invoke<StateView>("get_state").then((s) => (state = s));
invoke<Settings>("get_settings").then((s) => (mascotOn = s.mascot));
listen<Settings>("settings", (e) => (mascotOn = e.payload.mascot));
go("intent");
