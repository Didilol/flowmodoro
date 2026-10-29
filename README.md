# Flowmodoro

Âncora de atenção para TDAH: uma tarefa por vez, em passos pequenos, sem setup e sem alertas punitivos. App pessoal, para Mac e Windows, feito com Tauri.

## Rodar

```sh
cd app
npm install
npm run tauri dev
```

Precisa de Node e Rust ([rustup](https://rustup.rs)).

## Atalhos

| | Mac | Windows |
|---|---|---|
| Abrir a caixa | ⌥ Espaço | Ctrl Espaço |
| Perdi o foco | ⌥ F | Ctrl Alt F |
| Cabeça cheia | ⌥ D | Ctrl Alt D |
| Visão geral do dia | ⌥ H | Ctrl Alt H |
| Sons (myNoise) | ⌥ S | Ctrl Alt S |

## Gerar o app

- **Mac:** `npm run tauri build` dentro de `app/`. O instalador fica em `app/src-tauri/target/release/bundle/`.
- **Windows:** no GitHub, *Actions → Windows → Run workflow*. Ao terminar, o instalador fica nos *Artifacts* da execução. Também roda sozinho ao enviar uma tag `v*`.

## Pastas

- `app/`: o aplicativo (frontend em `src/`, núcleo em Rust em `src-tauri/`).
- `prototipo/`: protótipo HTML clicável usado para validar os fluxos.
- `MASCOTE-ANIMACAO.md`: como animar e exportar o mascote no After Effects (Lottie).
