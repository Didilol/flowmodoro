<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="site/assets/focora-horizontal-escuro.svg">
    <img src="site/assets/focora-horizontal-claro.svg" alt="Focora" width="360">
  </picture>
</p>

<p align="center"><strong>Âncora de foco para TDAH.</strong> Uma tarefa por vez, um passo de cada vez.<br>
Grátis para Mac e Windows · sem cadastro · seus dados ficam no seu computador.</p>

<p align="center">
  <a href="https://didilol.github.io/flowmodoro/"><strong>Baixar no site</strong></a> ·
  <a href="https://github.com/Didilol/flowmodoro/releases/latest">Última versão</a>
</p>

<p align="center"><img src="site/assets/ui/barra.svg" alt="A barra do Focora com o passo atual, o timer e o mascote espiando" width="440"></p>

> [!NOTE]
> **Feito por uma pessoa com TDAH, para as próprias necessidades.** Eu não tenho formação na área clínica nem em saúde: as funcionalidades nasceram da minha experiência, do que funcionou para mim. Decidi disponibilizar o Focora porque ele pode ajudar outras pessoas. **Ele não substitui acompanhamento profissional.** Se você tem ou suspeita ter TDAH, procure um médico, psiquiatra ou psicólogo.
>
> Sugestões são bem-vindas: [diogenes.edc@gmail.com](mailto:diogenes.edc@gmail.com).

## O que é

O Focora é uma barra pequena que fica por cima de qualquer programa e mostra **só o passo atual** da sua tarefa. Ele foi feito por uma pessoa com TDAH para quem trava diante de listas longas e de planejamento.

- **Zero configuração:** abriu, escreveu o que vai fazer, começou.
- **Um passo por vez:** nunca aparece uma lista longa.
- **Timer que conta para cima:** sem contagem regressiva nem alarme. Aos 5 minutos, a cor vira âmbar e o mascote põe os fones: você entrou em flow.
- **⚑ Perdi o foco:** o app pergunta com calma se a tarefa travou, sugere um passo menor ou guarda a tarefa para depois.
- **Cabeça cheia:** despeje os pensamentos numa gaveta, respire 30 segundos e volte para o passo atual.
- **Mascote:** trabalha do seu lado (*body doubling*) e nunca cobra.

### Por dentro

As telas abaixo são desenhadas com o mesmo CSS e o mesmo SVG do app (`site/ui/gerar.py`).

<p align="center">
  <img src="site/assets/ui/caixa-passo.svg" alt="A caixa central perguntando o primeiro passo minúsculo" width="560"><br>
  <em>Toda conversa acontece numa caixa central, aberta por atalho. Uma pergunta por vez.</em>
</p>
<p align="center">
  <img src="site/assets/ui/caixa-foco.svg" alt="A caixa perguntando com calma se a tarefa travou" width="560"><br>
  <em>Perdeu o foco? O app pergunta com calma. Cada resposta é um clique ou uma tecla de 1 a 9.</em>
</p>
<p align="center">
  <img src="site/assets/ui/barra-flow.svg" alt="A barra em flow, com o timer âmbar e o mascote de fones" width="440"><br>
  <em>Depois de 5 minutos de foco, o timer fica âmbar e o mascote põe os fones.</em>
</p>
<p align="center">
  <img src="site/assets/ui/menu.svg" alt="O menu do Focora na barra de menus" width="310"><br>
  <em>O resto mora no menu da barra de menus (ou da bandeja, no Windows).</em>
</p>

## Instalar

Baixe pelo [site](https://didilol.github.io/flowmodoro/) ou pela [última versão](https://github.com/Didilol/flowmodoro/releases/latest).

- **Mac (Apple Silicon, M1 ou mais novo):** abra o `.dmg` e arraste o Focora para **Aplicativos**.
- **Windows 10/11:** rode o `-setup.exe`.

A partir da versão 0.2.0, o Focora **se atualiza sozinho**. Ele baixa a versão nova em segundo plano, instala quando nenhum timer está rodando e reabre onde você parou. Cada atualização é assinada; o app recusa qualquer atualização sem a assinatura do autor.

> [!IMPORTANT]
> **Por que o Mac diz que o app não é verificado?**
> Para o macOS tratar um app como "verificado", o desenvolvedor precisa pagar a assinatura anual do programa de desenvolvedores da Apple (US$ 99 por ano). O Focora é um projeto pessoal e gratuito e ainda não tem essa assinatura. O aviso **não significa que a Apple encontrou um problema**: significa só que ela não analisou o app. O código está aqui para você conferir.
>
> **Para abrir:** abra o Focora uma vez (o macOS vai bloquear), vá em **Ajustes do Sistema → Privacidade e Segurança**, role até o fim e clique em **Abrir Mesmo Assim**. Isso só precisa ser feito uma vez. Pelo Terminal, dá no mesmo:
> ```sh
> xattr -dr com.apple.quarantine /Applications/Focora.app
> ```
> No Windows, se aparecer "O Windows protegeu o computador", clique em **Mais informações → Executar assim mesmo**.

## Privacidade e segurança

- **Seus dados ficam no seu computador.** Não há conta, nuvem, anúncios nem rastreamento. Tarefas, passos e anotações ficam num banco SQLite local:
  - Mac: `~/Library/Application Support/com.flowmodoro.app`
  - Windows: `%APPDATA%\com.flowmodoro.app`
- **O código está à vista.** Qualquer pessoa pode ler este repositório e conferir que o app não tem nada malicioso.
- **Quando o app usa a internet:** só na janela **Sons** (carrega o [myNoise](https://mynoise.net) e uma fonte do Google Fonts para o visual) e para **procurar atualizações** em `github.com/Didilol/flowmodoro/releases`. Nada sobre você ou suas tarefas é enviado.

## Atalhos

| | Mac | Windows |
|---|---|---|
| Abrir a caixa | ⌥ Espaço | Ctrl Espaço |
| Perdi o foco | ⌥ F | Ctrl Alt F |
| Cabeça cheia | ⌥ D | Ctrl Alt D |
| Pausar / continuar | ⌥ P | Ctrl Alt P |
| Visão geral do dia | ⌥ H | Ctrl Alt H |
| Sons (myNoise) | ⌥ S | Ctrl Alt S |

## Desenvolvimento

```sh
cd app
npm install
npm run tauri dev
```

Precisa de Node e Rust ([rustup](https://rustup.rs)). Pastas:

- `app/`: o aplicativo (frontend em `src/`, núcleo em Rust em `src-tauri/`).
- `site/`: o site de download, publicado no GitHub Pages. As telas do app em `site/ui/` e `site/assets/ui/` saem de `python3 site/ui/gerar.py`.

### Gerar uma versão

1. Suba a versão em `app/package.json`, `app/src-tauri/Cargo.toml` e `app/src-tauri/tauri.conf.json`.
2. **Mac** (dentro de `app/`), assinando o pacote de atualização com a chave em `~/.tauri/focora-updater.key`:
   ```sh
   export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/focora-updater.key)"
   export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(security find-generic-password -s focora-updater-key-password -w)"
   CI=true npm run tauri build
   ```
   O `CI=true` pula a etapa em que o Finder arruma a janela do `.dmg`, que pode travar esperando permissão.
3. Crie a release com o `.dmg`, o `Focora.app.tar.gz` e um `latest.json` com a entrada `darwin-aarch64`.
4. **Windows:** envie a tag (`git push origin vX.Y.Z`) ou rode *Actions → Windows → Run workflow*. O workflow anexa os instaladores e acrescenta a entrada `windows-x86_64` ao `latest.json`. Ele usa os secrets `TAURI_SIGNING_PRIVATE_KEY` e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

## Sugestões

Ideias, problemas ou sugestões: mande um e-mail para [diogenes.edc@gmail.com](mailto:diogenes.edc@gmail.com).

## Licença

Gratuito para uso pessoal. O código é público para leitura e verificação, mas **não é open source**: copiar, modificar ou redistribuir exige autorização. Veja [LICENSE](LICENSE).
