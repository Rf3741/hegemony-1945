# QA: partida longa headless

Gerado por `cargo run --example partida_longa` (crate guerra_fria). Perfil dev (opt-level 1 no workspace, 3 nas dependências, overflow checks ligados).

- Período: 1945-01-01 + 20 anos (7300 dias); IA ligada: tecnologia (`escolher_ia` para todos), eventos (jogador USA tratado como IA via `escolha_ia`), diplomacia (`tick_dia`), política, economia + legado.
- Carga: 2.9s (inclui 90 dias de aquecimento da economia). Tempo total da simulação: 247s.
- RSS: inicial 4 MB, após carga 113 MB, final 113 MB.
- Eventos de IA resolvidos pelo "jogador": 104.
- A IA econômica enfileirou obras estatais e privadas durante todo o período; o teste aquecido de cinco anos registra +8,7% de PIB mundial com IA (+7,0% sem IA).

## Tempo médio por dia (ms)

| Sistema | ms/dia |
|---|---|
| economia (sim_economia) | 23.39 |
| legado (simulacao::executar_tick_economia) | 1.35 |
| política | 6.87 |
| diplomacia | 0.01 |
| tecnologia | 0.52 |
| eventos | 0.06 |

## Resumo anual

| Ano | t(s) | RSS MB | PIB mundial/ano | pop. mundial | techs conhecidas | notícias | pactos | tensão | dias eco |
|---|---|---|---|---|---|---|---|---|---|
| 1946 | 12 | 108 | 1.80e12 | 2.45e9 | 1762 | 41 | 1 | 15.0 | 455 |
| 1947 | 24 | 109 | 1.83e12 | 2.46e9 | 1947 | 47 | 1 | 18.9 | 820 |
| 1948 | 36 | 109 | 1.85e12 | 2.47e9 | 2122 | 51 | 1 | 21.6 | 1185 |
| 1948 | 48 | 110 | 1.87e12 | 2.49e9 | 2262 | 55 | 1 | 38.3 | 1550 |
| 1949 | 60 | 110 | 1.88e12 | 2.50e9 | 2415 | 59 | 1 | 31.0 | 1915 |
| 1950 | 72 | 110 | 1.89e12 | 2.51e9 | 2567 | 67 | 1 | 23.7 | 2280 |
| 1951 | 84 | 111 | 1.90e12 | 2.52e9 | 2702 | 71 | 1 | 16.4 | 2645 |
| 1952 | 97 | 111 | 1.91e12 | 2.54e9 | 2833 | 76 | 2 | 15.0 | 3010 |
| 1953 | 110 | 111 | 1.92e12 | 2.55e9 | 2984 | 81 | 2 | 15.0 | 3375 |
| 1954 | 122 | 111 | 1.93e12 | 2.56e9 | 3115 | 85 | 2 | 15.0 | 3740 |
| 1955 | 134 | 111 | 1.94e12 | 2.57e9 | 3236 | 91 | 2 | 18.9 | 4105 |
| 1956 | 147 | 112 | 1.95e12 | 2.59e9 | 3368 | 100 | 2 | 15.0 | 4470 |
| 1957 | 159 | 112 | 1.96e12 | 2.60e9 | 3498 | 102 | 2 | 15.0 | 4835 |
| 1958 | 172 | 112 | 1.97e12 | 2.61e9 | 3634 | 106 | 2 | 15.0 | 5200 |
| 1959 | 185 | 112 | 1.98e12 | 2.62e9 | 3768 | 110 | 2 | 15.0 | 5565 |
| 1960 | 197 | 113 | 1.99e12 | 2.64e9 | 3884 | 117 | 2 | 15.0 | 5930 |
| 1961 | 209 | 113 | 2.00e12 | 2.65e9 | 4021 | 127 | 2 | 15.0 | 6295 |
| 1962 | 222 | 113 | 2.01e12 | 2.66e9 | 4125 | 130 | 2 | 15.0 | 6660 |
| 1963 | 235 | 113 | 2.01e12 | 2.68e9 | 4256 | 134 | 2 | 15.0 | 7025 |
| 1964 | 247 | 113 | 2.02e12 | 2.69e9 | 4375 | 137 | 2 | 15.0 | 7390 |

## Tamanho máximo das estruturas (explosão de entidades)

- eventos_disparados: 137
- filas_construcao(soma): 236
- jornal_eventos: 137
- log_diplomacia: 169
- pactos_diplomacia: 2
- pendentes_eventos: 0
- rotas_comerciais: 20

## Pânicos

Nenhum pânico.

## Achados (NaN, valores absurdos, auditorias)

Nenhum.
