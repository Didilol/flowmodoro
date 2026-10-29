// Injected into the myNoise window before the page runs: keeps only the player,
// the sliders and the presets, dressed in the Flowmodoro palette. Personal use only.
(() => {
  if (!location.hostname.endsWith("mynoise.net")) return;

  const SOUNDS = [
    ["Tempestade", "stormSoundGenerator"],
    ["Chuva na barraca", "campingRainNoiseGenerator"],
    ["Chuva na janela", "customRainOnWindow"],
    ["Chuva branca", "whiteRainNoiseGenerator"],
    ["Riacho", "healingWaterSoundscapeGenerator"],
    ["Costa irlandesa", "windSeaRainNoiseGenerator"],
  ];

  // Only the generator pages get trimmed; anything else (e.g. donating) shows as is.
  const generator = location.pathname.startsWith("/NoiseMachines/");
  if (generator) document.documentElement.classList.add("fm");

  // Slider handles fade from the app's teal to its "flow" gold.
  const handles = Array.from({ length: 10 }, (_, i) =>
    `html.fm #s${i} .ui-slider-handle { background: color-mix(in oklch, var(--fm-flow) ${Math.round((i / 9) * 100)}%, var(--fm-accent)) !important; }`,
  ).join("\n");

  const css = `
@import url("https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible:wght@400;700&family=Bricolage+Grotesque:opsz,wght@12..96,500..700&family=IBM+Plex+Mono&display=swap");
:root {
  --fm-bg: #1c2524; --fm-panel: #26312f; --fm-line: #33403e; --fm-btn: #2a3533; --fm-btn-hover: #364442;
  --fm-fg: #eef3f1; --fm-muted: #9aaaa7; --fm-accent: #5dbab0; --fm-flow: #e3b24c;
}
html.fm, html.fm body {
  background: var(--fm-bg) !important; color: var(--fm-fg) !important;
  font-family: "Atkinson Hyperlegible", system-ui, sans-serif !important; font-weight: 400 !important;
}
html.fm body > :not(.skeleton):not(#fm-head):not(#powerTip):not(script):not(style) { display: none !important; }
html.fm .skeleton { display: block !important; min-height: 0 !important; max-width: 560px; margin: 0 auto; padding: 0 16px 24px; }
html.fm .skeleton > :not(#player):not(.skeletonContents) { display: none !important; }
html.fm .nested { display: block !important; }
html.fm .nested > :not(#controls) { display: none !important; }
html.fm #controls > :not(:first-child) { display: none !important; }
html.fm #backwrapper, html.fm .skeleton a[href*="donate"], html.fm .actionlink[onclick*="login"] { display: none !important; }

html.fm .mainTitle {
  font-family: "Bricolage Grotesque", system-ui, sans-serif !important; font-weight: 700 !important;
  font-size: 24px !important; color: var(--fm-fg) !important; text-align: center; margin: 8px auto 0 !important;
}
html.fm .mixer { margin: 8px auto 0 !important; }
html.fm .ui-slider-vertical { background: var(--fm-line) !important; }
html.fm .ui-slider-handle {
  border: 3px solid var(--fm-bg) !important; box-shadow: 0 0 0 1px var(--fm-line);
  transition: box-shadow 0.15s, filter 0.15s;
}
html.fm .ui-slider-vertical:hover > .ui-slider-handle { border-width: 3px !important; box-shadow: 0 0 0 2px var(--fm-accent); }
html.fm .ui-slider-handle:active { filter: brightness(1.15); }
${handles}

html.fm .faBtn, html.fm .faBtnBig {
  background: var(--fm-btn) !important; border: 1px solid var(--fm-line) !important; color: var(--fm-fg) !important;
  border-radius: 10px !important; transition: background-color 0.15s, color 0.15s, transform 0.12s;
}
html.fm .faBtn:hover, html.fm .faBtnBig:hover { background: var(--fm-btn-hover) !important; color: var(--fm-accent) !important; transform: translateY(-1px); }
html.fm .faBtn:active, html.fm .faBtnBig:active { transform: scale(0.92); }
html.fm #mute { background: var(--fm-accent) !important; border-color: var(--fm-accent) !important; color: var(--fm-bg) !important; border-radius: 50% !important; }
html.fm #mute:hover { color: var(--fm-bg) !important; filter: brightness(1.08); }
html.fm #fftCanvas { border-color: var(--fm-accent) !important; }
html.fm .btnInput { background: var(--fm-btn) !important; border: 1px solid var(--fm-line) !important; color: var(--fm-fg) !important; border-radius: 6px; }
html.fm .msg { color: var(--fm-muted) !important; font-family: "IBM Plex Mono", ui-monospace, monospace !important; font-size: 12.5px !important; text-align: center; }

html.fm .nestedSection {
  background: var(--fm-panel) !important; border: 1px solid var(--fm-line) !important;
  box-shadow: none !important; border-radius: 14px !important; padding: 14px 18px !important;
}
html.fm #controls h2 { font-family: "Bricolage Grotesque", system-ui, sans-serif !important; font-size: 15px !important; color: var(--fm-fg) !important; margin: 0 0 6px !important; }
html.fm #controls p { line-height: 2 !important; margin: 0 !important; color: var(--fm-line); }
html.fm .actionlink { color: var(--fm-accent) !important; padding: 3px 7px !important; border-radius: 6px !important; transition: background-color 0.15s; }
html.fm .actionlink:hover { background: color-mix(in srgb, var(--fm-accent) 16%, transparent); }
html.fm span.white { color: var(--fm-flow) !important; }
html.fm #powerTip { background: var(--fm-panel) !important; color: var(--fm-fg) !important; border: 1px solid var(--fm-line); font-family: "Atkinson Hyperlegible", sans-serif !important; }

#fm-head {
  position: sticky; top: 0; z-index: 1000; background: var(--fm-bg); color: var(--fm-fg);
  padding: 14px 16px 10px; border-bottom: 1px solid var(--fm-line);
  font: 400 14px/1.4 "Atkinson Hyperlegible", system-ui, sans-serif;
}
#fm-head .fm-top { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 8px; }
#fm-head .fm-eyebrow { font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fm-accent); }
#fm-head .fm-support { font-size: 12px; color: var(--fm-muted); text-decoration: none; }
#fm-head .fm-support:hover { color: var(--fm-flow); }
#fm-head .fm-chips { display: flex; flex-wrap: wrap; gap: 6px; }
#fm-head .fm-chips a {
  color: var(--fm-fg); text-decoration: none; font-size: 13px; padding: 5px 11px; border-radius: 999px;
  border: 1px solid var(--fm-line); background: var(--fm-btn); transition: background-color 0.15s, border-color 0.15s, transform 0.12s;
}
#fm-head .fm-chips a:hover { border-color: var(--fm-accent); background: var(--fm-btn-hover); }
#fm-head .fm-chips a:active { transform: scale(0.96); }
#fm-head .fm-chips a.on { background: var(--fm-accent); border-color: var(--fm-accent); color: var(--fm-bg); font-weight: 700; }
`;

  const style = document.createElement("style");
  style.id = "fm-skin";
  style.textContent = css;
  (document.head || document.documentElement).appendChild(style);

  document.addEventListener("DOMContentLoaded", () => {
    // Move to the end so it wins over the page's own <style> blocks.
    document.head.appendChild(style);

    const head = document.createElement("div");
    head.id = "fm-head";
    const chips = SOUNDS.map(([label, page]) => {
      const on = location.pathname.includes(page) ? ' class="on"' : "";
      return `<a href="/NoiseMachines/${page}.php"${on}>${label}</a>`;
    }).join("");
    head.innerHTML = `
      <div class="fm-top">
        <span class="fm-eyebrow">Sons · myNoise</span>
        <a class="fm-support" href="/donate.php" title="O myNoise vive de doações">apoiar o autor ♥</a>
      </div>
      <div class="fm-chips">${chips}</div>`;
    document.body.prepend(head);
  });
})();
