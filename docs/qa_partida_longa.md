# QA: partida longa headless

Gerado por `cargo run --example partida_longa` (crate guerra_fria). Perfil dev (opt-level 1 no workspace, 3 nas dependências, overflow checks ligados).

- Período: 1945-01-01 + 20 anos (7300 dias); IA ligada: tecnologia (`escolher_ia` para todos), eventos (jogador USA tratado como IA via `escolha_ia`), diplomacia (`tick_dia`), política, economia + legado.
- Carga: 6.1s (inclui 90 dias de aquecimento da economia). Tempo total da simulação: 368s.
- RSS: inicial 3 MB, após carga 94 MB, final 94 MB.
- Eventos de IA resolvidos pelo "jogador": 70.

## Tempo médio por dia (ms)

| Sistema | ms/dia |
|---|---|
| economia (sim_economia) | 38.55 |
| legado (simulacao::executar_tick_economia) | 1.62 |
| política | 7.69 |
| diplomacia | 0.02 |
| tecnologia | 0.63 |
| eventos | 0.03 |

## Resumo anual

| Ano | t(s) | RSS MB | PIB mundial/ano | pop. mundial | techs conhecidas | notícias | pactos | tensão | dias eco |
|---|---|---|---|---|---|---|---|---|---|
| 1946 | 25 | 106 | 3.74e12 | 3.55e9 | 1756 | 7 | 1 | 15.0 | 455 |
| 1947 | 51 | 91 | 3.76e12 | 3.56e9 | 1942 | 13 | 1 | 18.9 | 820 |
| 1948 | 75 | 92 | 3.78e12 | 3.58e9 | 2111 | 17 | 1 | 21.6 | 1185 |
| 1948 | 95 | 92 | 3.79e12 | 3.60e9 | 2244 | 21 | 1 | 38.3 | 1550 |
| 1949 | 112 | 92 | 3.80e12 | 3.62e9 | 2389 | 25 | 1 | 31.0 | 1915 |
| 1950 | 127 | 92 | 3.81e12 | 3.64e9 | 2530 | 33 | 1 | 23.7 | 2280 |
| 1951 | 144 | 92 | 3.82e12 | 3.66e9 | 2658 | 37 | 1 | 16.4 | 2645 |
| 1952 | 161 | 92 | 3.83e12 | 3.68e9 | 2783 | 42 | 2 | 15.0 | 3010 |
| 1953 | 177 | 93 | 3.84e12 | 3.70e9 | 2919 | 47 | 2 | 15.0 | 3375 |
| 1954 | 195 | 93 | 3.85e12 | 3.72e9 | 3034 | 51 | 2 | 15.0 | 3740 |
| 1955 | 213 | 93 | 3.86e12 | 3.74e9 | 3142 | 57 | 2 | 18.9 | 4105 |
| 1956 | 231 | 93 | 3.86e12 | 3.76e9 | 3249 | 66 | 2 | 15.0 | 4470 |
| 1957 | 249 | 93 | 3.87e12 | 3.78e9 | 3378 | 68 | 2 | 15.0 | 4835 |
| 1958 | 269 | 93 | 3.88e12 | 3.80e9 | 3500 | 72 | 2 | 15.0 | 5200 |
| 1959 | 286 | 93 | 3.88e12 | 3.82e9 | 3609 | 76 | 2 | 15.0 | 5565 |
| 1960 | 303 | 93 | 3.89e12 | 3.84e9 | 3727 | 83 | 2 | 15.0 | 5930 |
| 1961 | 319 | 93 | 3.89e12 | 3.87e9 | 3837 | 93 | 2 | 15.0 | 6295 |
| 1962 | 338 | 94 | 3.90e12 | 3.89e9 | 3950 | 96 | 2 | 15.0 | 6660 |
| 1963 | 353 | 94 | 3.91e12 | 3.91e9 | 4054 | 100 | 2 | 15.0 | 7025 |
| 1964 | 368 | 94 | 3.91e12 | 3.93e9 | 4170 | 103 | 2 | 15.0 | 7390 |

## Tamanho máximo das estruturas (explosão de entidades)

- eventos_disparados: 103
- filas_construcao(soma): 133
- jornal_eventos: 103
- log_diplomacia: 169
- pactos_diplomacia: 2
- pendentes_eventos: 0
- rotas_comerciais: 20

## Pânicos

Nenhum pânico.

## Achados (NaN, valores absurdos, auditorias)

Nenhum.


## Desempenho (economia, 120 dias, rayon por mercado/país)

Antes (1 thread): 36,6 ms/dia. Depois (8 threads): 20,6 ms/dia (produção 14,2 para 8,6; distribuição 9,1 para 3,8; consumo 8,6 para 3,2). O hash do estado é idêntico com 1 e 8 threads (determinístico). Medido com a máquina carregada por outros agentes.
Observações: a IA não constrói (fila de obras estável em 133), poucos pactos diplomáticos em 20 anos, PIB mundial quase parado (+0,2%/ano).
