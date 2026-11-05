# Modo smoke (`--smoke`)

Executa o jogo sozinho, sem interação humana, para teste automatizado de integração:
abre uma partida nova como os EUA, na velocidade máxima, e encerra depois de N segundos
reais imprimindo uma linha de resultado no stdout.

Toda a lógica está em `crates/guerra_fria/src/plugins/smoke.rs`. Os sistemas só são
registrados quando `--smoke` está presente; sem a bandeira o binário não executa nada
deste plugin (custo zero no jogo normal).

## Uso

```sh
cargo run -p guerra_fria -- --smoke 60
cargo run -p guerra_fria -- --smoke 60 --qualidade ultra --painel 6
cargo run -p guerra_fria -- --smoke 30 --screenshot /tmp/fim.png
```

### Opções

| Bandeira | Valor | Efeito |
| --- | --- | --- |
| `--smoke <segundos>` | número real > 0 | Ativa o modo. Pula o menu, inicia partida como `USA` (`PAIS_SMOKE`), sem tutorial, velocidade `V5` (a máxima) e encerra após os segundos indicados. |
| `--screenshot <arquivo.png>` | caminho com extensão `.png` | Pedida 2 s antes do encerramento (`ANTECEDENCIA_CAPTURE`). Exige `--smoke`. |
| `--qualidade <baixo\|medio\|alto\|ultra>` | um dos quatro níveis | Força a qualidade gráfica daquela execução. Aceita maiúsculas e `médio` sem acento. **Não** exige `--smoke` e **não** é gravada nas configurações. |
| `--painel <1-9>` | o mesmo número do atalho no jogo | Abre o painel já no início da partida, para exercitar a UI dele. Exige `--smoke`. |
| `--camera <lon> <lat>` | coordenadas geográficas em graus | Posiciona a câmera nas coordenadas dadas. Exige `--smoke`. |
| `--zoom <fator>` | fator real > 0 | Define a escala/zoom da câmera ortográfica. Exige `--smoke`. |

Os números de painel são os atalhos do jogo (ver `docs/ESTADO_ATUAL.md`):
1 país · 2 mercado · 3 comércio · 4 diplomacia · 5 tecnologia · 6 guerra ·
7 rankings/estatísticas · 8 enciclopédia · 9 áudio (aba Áudio do menu de opções).

Qualquer outra bandeira desconhecida (começada por `-`) é rejeitada antes de abrir a
janela, com mensagem no stderr e código de saída 2.

## Saída e códigos

No encerramento, uma linha no stdout:

```
SMOKE OK dias=1723 fps_medio=58.7 fps_min=41.2
```

* `dias`: dias simulados percorridos (`RecursoRelogio::total_ticks` desde 1945-01-01).
* `fps_medio` / `fps_min`: média e mínimo, amostrados quadro a quadro a partir do
  `FrameTimeDiagnosticsPlugin` do Bevy (a janela histórica do diagnóstico é de apenas
  120 quadros, por isso a acumulação é própria). Sem amostra do diagnóstico, a média
  recai sobre quadros contados / tempo decorrido e o mínimo iguala a média.

Códigos de saída (`main` devolve o `AppExit` do Bevy, que vira o código do processo):

| Código | Quando |
| --- | --- |
| `0` | `SMOKE OK` impresso. |
| `1` | `SMOKE FALHA motivo=partida_nao_iniciada ...` no stderr: o prazo expirou sem que a partida tivesse começado (carregamento travado). |
| `2` | Argumentos inválidos, antes de criar a janela. |
| ≠ 0 | Pânico: o processo morre com o código padrão do Rust (101) e nenhuma linha `SMOKE OK` é impressa. |

## Detalhes de implementação

* O prazo é medido com `Time<Real>::elapsed_secs_f64()`, ancorado no primeiro quadro
  (`Contagem::marco`), de modo que o processo termine mesmo se o carregamento emperrar.
* A partida começa em `OnEnter(AppState::Menu)` — estado que o jogo só alcança quando a
  economia e a textura do mapa terminam — inserindo `PaisJogador` e indo direto para
  `AppState::Jogando`, como faz o botão "Novo jogo" do menu.
* `manter_relogio_no_maximo` reimpõe `V5` e o despause a cada quadro: eventos e
  formações pausam o tempo, o que travaria a contagem de dias.
* `reabrir_painel` reabre o painel escolhido durante os primeiros 240 quadros, porque
  os recursos de alguns painéis só existem depois que cada simulador termina de carregar.
* A qualidade é aplicada no `ConfigJogo` lido em `main.rs` e reinserida como recurso
  depois de `ConfigPlugin`; o terreno, os objetos e a atmosfera leem todos o mesmo
  `graficos.qualidade`. O atraso de gravação (`salvar_com_atraso`) tem proteção na
  primeira execução de `Update`, então a sobreposição nunca chega ao arquivo do usuário.
* A captura usa a API de screenshot do Bevy 0.20: um componente `Screenshot` no mundo
  com um observador `save_to_disk`, i.e.
  `commands.spawn(Screenshot::primary_window()).observe(save_to_disk(caminho))`.

## Requisitos

* Precisa de uma sessão gráfica real (janela + GPU). `--screenshot` captura o backbuffer
  da janela primária; em headless não há janela e o teste não faz sentido.
* O `FrameTimeDiagnosticsPlugin` é adicionado pelo próprio `SmokePlugin` (ele não faz
  parte do `DefaultPlugins`).

## Testes

`cargo test -p guerra_fria -- --test-threads=1` roda os testes de parsing de
`plugins::smoke::testes`: bandeiras ausentes, conjunto completo, `--qualidade` isolada,
decimais e caixa alta, dependências entre bandeiras, valores fora da faixa e bandeiras
desconhecidas. O binário do jogo não foi executado durante o desenvolvimento deste modo.
