# Mascote: como animar no After Effects e exportar

O app toca as animações com o **lottie-web** (renderizador SVG, versão leve). Tudo abaixo existe para que o que você anima no AE apareça igual no app e gaste o mínimo de máquina.

## 1. Composição

| | |
|---|---|
| Tamanho | **512 × 512 px** (é vetor; o tamanho só define o espaço de trabalho) |
| Frame rate | **24 fps** |
| Fundo | transparente (a cor da comp não é exportada) |
| No app | a comp aparece com **96 × 96 px**, ou seja, 1 px da comp ≈ 0,19 px na tela |

**Linha da barra:** o mascote fica atrás da barra flutuante, espiando por cima.

- Crie uma camada-guia (retângulo de `y = 384` até o fim da comp) e marque **Layer → Guide Layer**. Ela não é exportada.
- Tudo acima de `y = 384` aparece sobre a barra (cerca de 72 px na tela). O que fica abaixo da linha só aparece na caixa central, na animação de terminar.
- Desenhe o corpo inteiro mesmo assim, inclusive abaixo da linha, porque na caixa ele aparece completo.

**Posição e área segura:**

- Centralize o mascote em `x = 256`. O corpo deve ter por volta de **300 px de largura**, o que dá uns 56 px na tela.
- Deixe pelo menos 24 px livres no topo da comp para o broto e para os pulos.
- Os pulos devem subir no máximo uns 60 px.

**Patinhas:** as patas ficam **na frente** da barra. Coloque-as numa camada (ou pré-comp) chamada exatamente **`.patas`**, com o ponto no começo. Elas devem cruzar a linha da barra, mais ou menos de `y = 360` a `y = 410`. O app usa esse nome para desenhar só as patas na frente da barra e o resto atrás.

## 2. As animações (uma comp por arquivo)

Todas as animações de um mesmo grupo **começam e terminam na mesma pose de descanso**, frame idêntico. Assim o app troca de uma para outra sem pulo.

| Arquivo | Quando toca | Duração | Observação |
|---|---|---|---|
| `trabalhando.json` | durante a tarefa | 2–4 s | Pose de descanso + uma variação leve (respirar, piscar, olhar para o lado). O app **segura o frame 0** e toca a variação de vez em quando (a cada 8–15 s). |
| `flow-entrar.json` | ao chegar em 5 min de foco | 1–1,5 s | Põe os fones. Termina na pose de descanso do flow. |
| `flow.json` | em flow | 2–4 s | Igual ao `trabalhando`, mas de fones. Mesma regra do frame 0. |
| `comemorar.json` | ✓ concluiu um passo | 1–1,2 s | Curto e alegre. |
| `olhar.json` | ⚑ Perdi o foco / Cabeça cheia | ~3 s | Olha para você com calma, segura e volta. Sem julgamento. |
| `respirar.json` | respiração da Cabeça cheia | **exatamente 10 s (240 frames)** | Frames 0–96 inspira (4 s), 96–240 expira (6 s). Olhos fechados. O app toca uma vez por ciclo, sincronizado com o círculo. |
| `terminar.json` | caixa, ao terminar a tarefa | 2–3 s | Comemoração maior com o corpo inteiro à mostra. **O último frame fica parado** na tela. |

Opcionais:

- `piscar.json` (6–8 frames): uma piscada solta, que o app toca a cada 4–8 s. Sem ele, a piscada fica dentro da variação de `trabalhando` e `flow`.
- `comemorar-flow.json` e `olhar-flow.json`: versões com fones. Sem elas, o mascote tira os fones durante essas animações e volta com eles depois.

**Por que quase tudo fica parado:** uma animação em loop gasta processamento o tempo todo. Segurando o frame 0 e animando só de vez em quando, o mascote parece vivo e quase não consome nada enquanto você usa AE, Photoshop e Illustrator.

## 3. O que pode e o que não pode

O player do app é leve e **não roda expressões nem efeitos**.

**Pode:**

- Camadas de forma (shape layers): path, elipse, retângulo, estrela, fill, stroke e gradientes
- Trim Paths, Repeater e Round Corners
- Parenting, nulls e pré-comps
- Máscaras simples (Add/Subtract, sem feather) e track mattes (alpha/luma), estes com moderação
- Qualquer easing nos keyframes, inclusive o Graph Editor

**Não pode** (não aparece ou quebra no app):

- **Efeitos** (Glow, Drop Shadow, Turbulent Displace…) e **Layer Styles**
- **Expressões**: converta antes com **Animation → Keyframe Assistant → Convert Expression to Keyframes**. Rigs feitos com expressões (Duik, RubberHose) precisam passar por essa conversão. Teste no preview depois.
- **Puppet Tool**
- **Merge Paths**
- Camadas 3D, câmera, luzes e motion blur
- Blending modes (use opacidade)
- Imagens ou camadas raster, e texto (converta em formas)

**Do Illustrator:** importe o `.ai` como footage e use **Layer → Create → Create Shapes from Vector Layer**. Depois apague a camada original.

**Leveza:**

- Menos de ~30 camadas por comp.
- Paths com poucos pontos.
- Arquivo final abaixo de ~100 KB.

## 4. Cores do app

| | |
|---|---|
| Corpo | `#5DBAB0` |
| Fones (e cor do flow) | `#E3B24C` |
| Olhos e contornos | `#17201F` |
| Folha do broto | `#8FD18A` |
| Bochechas | `#F39A8A` a 55% |

A barra é escura (`#1C2524`) e a caixa pode ser clara ou escura. Um contorno fino ou uma sombra desenhada como forma ajuda o mascote a se destacar nos dois fundos.

## 5. Exportar

1. Instale o **Bodymovin**, que é gratuito (aescripts.com/bodymovin ou o plugin *LottieFiles for After Effects*).
2. No AE: **Settings → Scripting & Expressions → "Allow Scripts to Write Files and Access Network"** ligado.
3. Dê às comps exatamente os nomes da tabela, sem `.json`: `trabalhando`, `flow-entrar`, `flow`, `comemorar`, `olhar`, `respirar`, `terminar`.
4. **Window → Extensions → Bodymovin**: marque as comps e, em *Destination Folder*, aponte para:
   `/Volumes/SSD/Flowmodoro/app/public/mascote/`
5. Na engrenagem (*Settings*) de cada comp: **Standard**, sem glyphs e sem imagens. Deixe o resto como está.
6. **Render.**
7. Confira o resultado no preview do próprio Bodymovin ou arrastando o `.json` para lottiefiles.com/preview, com o renderizador **SVG**. É o mesmo motor do app: o que aparecer lá é o que vai aparecer na barra.

Não precisa exportar tudo de uma vez. Me avise quando o primeiro arquivo estiver na pasta: eu ligo o player e cada estado que ainda não tiver arquivo continua usando o mascote provisório.
