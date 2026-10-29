// The mascot: body doubling without anyone watching. It never asks for anything;
// it just works alongside and reacts to what happens.
//
// Placeholder art in SVG. Each state is a CSS class on the root, so the final
// After Effects animations (Lottie, one file per state) can replace these one by one.

/** Loops: the mascot stays in these until something changes. */
export type BaseState = "work" | "flow" | "idle";
/** One-shots and holds layered on top of the base. */
export type Moment = "cheer" | "look" | "finish" | "inhale" | "exhale";

const BODY = `
<svg class="m-body" viewBox="0 -8 64 68" aria-hidden="true">
  <g class="m-sprout">
    <path d="M32 7 C32 3 31.5 0 30 -3" fill="none" stroke="var(--m-leaf)" stroke-width="2.2" stroke-linecap="round" />
    <path d="M30.5 -2 C33 -7 39 -8 42 -5 C39 -1 34 0 30.5 -2 Z" fill="var(--m-leaf)" />
  </g>
  <path class="m-shape" d="M6 60 V32 C6 15 18 6 32 6 C46 6 58 15 58 32 V60 Z" fill="var(--m-fill)" />
  <ellipse cx="32" cy="52" rx="17" ry="12" fill="#fff" opacity="0.14" />
  <g class="m-phones">
    <path d="M9 31 C9 9 55 9 55 31" fill="none" stroke="var(--m-phones)" stroke-width="3.4" stroke-linecap="round" />
    <rect x="3.5" y="25" width="8" height="13" rx="4" fill="var(--m-phones)" />
    <rect x="52.5" y="25" width="8" height="13" rx="4" fill="var(--m-phones)" />
  </g>
  <g class="m-eyes">
    <g class="m-pupils">
      <ellipse cx="23" cy="30" rx="3.4" ry="4.2" fill="var(--m-ink)" />
      <ellipse cx="41" cy="30" rx="3.4" ry="4.2" fill="var(--m-ink)" />
      <circle cx="24.2" cy="28.4" r="1.1" fill="#fff" />
      <circle cx="42.2" cy="28.4" r="1.1" fill="#fff" />
    </g>
  </g>
  <g class="m-happy" fill="none" stroke="var(--m-ink)" stroke-width="2.4" stroke-linecap="round">
    <path d="M19 31.5 Q23 26 27 31.5" />
    <path d="M37 31.5 Q41 26 45 31.5" />
  </g>
  <circle cx="16.5" cy="38" r="3.2" fill="var(--m-cheek)" />
  <circle cx="47.5" cy="38" r="3.2" fill="var(--m-cheek)" />
  <path class="m-mouth" d="M29 38.5 Q32 41 35 38.5" fill="none" stroke="var(--m-ink)" stroke-width="2" stroke-linecap="round" />
  <path class="m-mouth-open" d="M28 37.5 Q32 45 36 37.5 Z" fill="var(--m-ink)" />
</svg>`;

/** In the bar the paws rest on its top edge, in front of it, while the body hides behind. */
const PAWS = `
<svg class="m-paws" viewBox="0 0 64 10" aria-hidden="true">
  <rect x="13" y="0.5" width="13" height="8.5" rx="4.25" fill="var(--m-fill)" stroke="var(--m-edge)" stroke-width="1" />
  <rect x="38" y="0.5" width="13" height="8.5" rx="4.25" fill="var(--m-fill)" stroke="var(--m-edge)" stroke-width="1" />
</svg>`;

const ONE_SHOT_MS: Partial<Record<Moment, number>> = { cheer: 1100, look: 3000, finish: 2400 };
const MOMENTS: Moment[] = ["cheer", "look", "finish", "inhale", "exhale"];

export class Mascot {
  private base: BaseState = "idle";
  private moment: Moment | null = null;
  private timer: number | undefined;
  private blinkTimer: number | undefined;
  private enabled = true;

  constructor(private root: HTMLElement, withPaws: boolean) {
    root.classList.add("mascot", "m-idle");
    root.innerHTML = BODY + (withPaws ? PAWS : "");
    this.scheduleBlink();
    root.addEventListener("click", () => this.play("cheer"));
  }

  setBase(state: BaseState) {
    if (state === this.base) return;
    this.root.classList.replace(`m-${this.base}`, `m-${state}`);
    this.base = state;
  }

  /** Off: hidden and fully idle (no blink timer either). */
  setEnabled(on: boolean) {
    if (on === this.enabled) return;
    this.enabled = on;
    this.root.hidden = !on;
    this.release();
    clearTimeout(this.blinkTimer);
    if (on) this.scheduleBlink();
  }

  /** A moment that ends by itself (cheer, look, finish). */
  play(moment: Moment) {
    if (!this.enabled) return;
    this.set(moment);
    const ms = ONE_SHOT_MS[moment];
    if (ms) this.timer = window.setTimeout(() => this.release(), ms);
  }

  /** A moment that lasts until released or replaced (breathing). */
  hold(moment: Moment) {
    if (!this.enabled) return;
    this.set(moment);
  }

  release() {
    clearTimeout(this.timer);
    this.moment = null;
    this.root.classList.remove(...MOMENTS.map((m) => `m-${m}`));
  }

  private set(moment: Moment) {
    this.release();
    this.moment = moment;
    // Restart the animation even when the same moment repeats.
    void this.root.offsetWidth;
    this.root.classList.add(`m-${moment}`);
  }

  /** Slow, irregular blinks: alive, but nothing to look at. */
  private scheduleBlink() {
    this.blinkTimer = window.setTimeout(() => {
      // The box re-renders often; a mascot that left the page stops blinking.
      if (!this.root.isConnected || !this.enabled) return;
      if (!this.moment) {
        this.root.classList.add("m-blink");
        window.setTimeout(() => this.root.classList.remove("m-blink"), 160);
      }
      this.scheduleBlink();
    }, 3500 + Math.random() * 4500);
  }
}
