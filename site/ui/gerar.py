"""Gera as telas do Focora para o site e o README a partir do código real do app.

Lê app/src/styles.css e o SVG do mascote em app/src/mascot.ts e monta:
  site/ui/<tela>.html        páginas estáticas mostradas em iframes no site
  site/assets/ui/<tela>.svg  as mesmas telas em SVG (foreignObject) para o README do GitHub

Rodar da raiz do repositório depois de mudar a interface:  python3 site/ui/gerar.py
"""
import base64
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[2]
FONTS = ROOT / "site/assets/fonts"
CSS_SRC = (ROOT / "app/src/styles.css").read_text()
MASCOT_SRC = (ROOT / "app/src/mascot.ts").read_text()

FACES = [
    ("Atkinson Hyperlegible", "400", "atkinson-hyperlegible-latin-400-normal.woff2"),
    ("Atkinson Hyperlegible", "700", "atkinson-hyperlegible-latin-700-normal.woff2"),
    ("Bricolage Grotesque Variable", "200 800", "bricolage-grotesque-latin-wght-normal.woff2"),
    ("IBM Plex Mono", "400", "ibm-plex-mono-latin-400-normal.woff2"),
    ("IBM Plex Mono", "500", "ibm-plex-mono-latin-500-normal.woff2"),
]


def font_faces(inline: bool) -> str:
    out = []
    for family, weight, file in FACES:
        if inline:
            src = "data:font/woff2;base64," + base64.b64encode((FONTS / file).read_bytes()).decode()
        else:
            src = f"../assets/fonts/{file}"
        out.append(f'@font-face{{font-family:"{family}";font-weight:{weight};font-display:swap;src:url({src}) format("woff2")}}')
    return "\n".join(out)


def app_css(inline: bool) -> str:
    css = re.sub(r"^@import .*;$", "", CSS_SRC, flags=re.M)
    return font_faces(inline) + "\n" + css


def mascot(state: str) -> str:
    body = re.search(r"const BODY = `(.*?)`;", MASCOT_SRC, re.S).group(1)
    paws = re.search(r"const PAWS = `(.*?)`;", MASCOT_SRC, re.S).group(1)
    return f'<div class="mascot {state}">{body}{paws}</div>'


ICONS = {
    "pause": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round"><path d="M9 6v12M15 6v12"/></svg>',
    "done": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>',
    "lost": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 21V4"/><path d="M5 4h11l-2 4 2 4H5"/></svg>',
    "full": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9.5c2.2-2 3.8-2 6 0s3.8 2 6 0 3.8-2 6 0"/><path d="M3 15.5c2.2-2 3.8-2 6 0s3.8 2 6 0 3.8-2 6 0"/></svg>',
}


def bar(step: str, time: str, fill: int, flow: bool) -> str:
    """A barra flutuante, com a mesma marcação de app/index.html."""
    btns = "".join(f'<button class="ibtn" aria-label="{label}">{ICONS[k]}</button>'
                   for k, label in [("pause", "Pausar"), ("done", "Concluí este passo"), ("lost", "Perdi o foco"), ("full", "Cabeça cheia")])
    return (f'<div class="bar-body" style="height:140px">'
            f'<div class="bar-wrap">{mascot("m-flow" if flow else "m-work")}'
            f'<div class="bar{" flow" if flow else ""}"><div class="bar-main">'
            f'<div class="bar-step"><span class="step-text">{step}</span></div>'
            f'<span class="bar-time">{time}</span>{btns}</div>'
            f'<div class="track"><div class="fill" style="width:{fill}%"></div></div></div></div></div>')


def key(n: int) -> str:
    return f'<span class="key">{n}</span>'


def box(inner: str) -> str:
    """A caixa central, com o mesmo card de app/entry.html."""
    return f'<div class="entry-body"><div class="entry-card">{inner}</div></div>'


# Telas: (nome, largura, altura, html)
SCREENS = [
    ("barra", 440, 140, bar("Esboçar a cena 2", "3:12", 32, flow=False)),
    ("barra-flow", 440, 140, bar("Esboçar a cena 2", "7:45", 62, flow=True)),
    ("caixa-passo", 560, 290, box(
        '<p class="entry-task">Tarefa: <b>Vinheta para o portfólio</b></p>'
        '<h1><label>Qual o primeiro passo minúsculo?</label></h1>'
        '<input class="field" placeholder="Ex.: abrir a planilha" value="Abrir o projeto no After Effects"/>'
        '<button class="linkbtn" type="button">+ adicionar mais passos</button>'
        '<p class="hint">Algo que dá para começar em menos de 2 minutos. Enter começa.</p>')),
    ("caixa-foco", 560, 210, box(
        '<h1>Tudo bem. A tarefa travou ou você quer mudar de assunto?</h1>'
        f'<div class="row"><button class="btn" type="button">{key(1)}Travou</button>'
        f'<button class="btn" type="button">{key(2)}Mudar de assunto</button>'
        f'<button class="btn quiet" type="button">{key(3)}Voltar ao passo</button></div>')),
]

MENU_CSS = """
:root { color-scheme: light dark; }
.menu { font: 14px/1.2 -apple-system, "Segoe UI", system-ui, sans-serif; width: 290px; margin: 8px;
  background: rgb(246 247 247 / 0.96); color: #1d1f1f; border: 1px solid rgb(0 0 0 / 0.12);
  border-radius: 10px; padding: 5px; box-shadow: 0 10px 30px rgb(0 0 0 / 0.22); }
.menu div { display: flex; justify-content: space-between; padding: 5px 10px; border-radius: 5px; }
.menu div:first-child { background: #2b776f; color: #fff; }
.menu hr { border: 0; border-top: 1px solid rgb(0 0 0 / 0.1); margin: 5px 8px; }
.menu .k { color: rgb(0 0 0 / 0.45); }
.menu div:first-child .k { color: rgb(255 255 255 / 0.8); }
@media (prefers-color-scheme: dark) {
  .menu { background: rgb(40 44 44 / 0.96); color: #eef1f0; border-color: rgb(255 255 255 / 0.12); }
  .menu hr { border-top-color: rgb(255 255 255 / 0.12); } .menu .k { color: rgb(255 255 255 / 0.5); }
}"""
MENU = ('<div class="menu">'
        '<div><span>Abrir caixa</span><span class="k">⌥ Espaço</span></div>'
        '<div><span>Visão geral do dia</span><span class="k">⌥ H</span></div>'
        '<div><span>Sons</span><span class="k">⌥ S</span></div>'
        '<div><span>✓ Mascote</span><span></span></div>'
        '<div><span>Timer</span><span class="k">▸</span></div><hr>'
        '<div><span>Sair do Focora</span><span></span></div></div>')


def page(title: str, css: str, html: str) -> str:
    return (f'<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>{title}</title>'
            f'<meta name="robots" content="noindex"><style>{css}\nhtml,body{{background:transparent}}</style></head>'
            f'<body>{html}</body></html>\n')


# What <body> gives the app's pages; a foreignObject has no body, so its root gets it.
ROOT_CSS = (".ui-root{font-family:var(--f-body);font-size:15px;line-height:1.5;color:var(--ink);"
            "-webkit-font-smoothing:antialiased}"
            # WebKit skips painting a position:relative box inside a foreignObject.
            ".ui-root .bar-step{position:static}")


def xhtml(html: str) -> str:
    """Inside a foreignObject the markup is XML: inner SVGs need their namespace, void tags must close."""
    html = re.sub(r"<svg(?![^>]*xmlns)", '<svg xmlns="http://www.w3.org/2000/svg"', html)
    return re.sub(r"<(hr|br)>", r"<\1/>", html)


def svg(w: int, h: int, css: str, html: str) -> str:
    html = xhtml(html)
    css = css + "\n" + ROOT_CSS
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">'
            f'<style>{css}</style><foreignObject x="0" y="0" width="{w}" height="{h}">'
            f'<div xmlns="http://www.w3.org/1999/xhtml" class="ui-root" style="width:{w}px;height:{h}px">{html}</div>'
            f'</foreignObject></svg>\n')


def main():
    out_html = ROOT / "site/ui"
    out_svg = ROOT / "site/assets/ui"
    out_svg.mkdir(parents=True, exist_ok=True)
    css_link, css_inline = app_css(False), app_css(True)
    for name, w, h, html in SCREENS:
        (out_html / f"{name}.html").write_text(page(f"Focora · {name}", css_link, html))
        (out_svg / f"{name}.svg").write_text(svg(w, h, css_inline, html))
    (out_html / "menu.html").write_text(page("Focora · menu", MENU_CSS, MENU))
    (out_svg / "menu.svg").write_text(svg(310, 230, MENU_CSS, MENU))
    print("ok:", ", ".join(n for n, *_ in SCREENS), "+ menu")


main()
