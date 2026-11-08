# QA: partida longa headless

Gerado por `cargo run --example partida_longa` (crate guerra_fria). Perfil dev (opt-level 1 no workspace, 3 nas dependências, overflow checks ligados).

- Período: 1945-01-01 + 20 anos (7300 dias); IA ligada: tecnologia (`escolher_ia` para todos), eventos (jogador USA tratado como IA via `escolha_ia`), diplomacia (`tick_dia`), política, economia + legado; descolonização/formações (`Formacoes::processar`, integradas em economia, política e diplomacia como no cliente).
- Carga: 4.6s (inclui 90 dias de aquecimento da economia). Tempo total da simulação: 496s.
- RSS: inicial 4 MB, após carga 161 MB, final 161 MB.
- Eventos de IA resolvidos pelo "jogador": 76.

## Tempo médio por dia (ms)

| Sistema | ms/dia |
|---|---|
| economia (sim_economia) | 54.85 |
| legado (simulacao::executar_tick_economia) | 1.89 |
| formações (descolonização) | 0.05 |
| política | 7.64 |
| diplomacia | 0.02 |
| tecnologia | 1.32 |
| eventos | 0.08 |

## Resumo anual

| Ano | t(s) | RSS MB | s/ano | ms/dia | PIB mundial/ano | pop. mundial | techs conhecidas | notícias | pactos | tensão | dias eco |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1945 | 28 | 122 | 28 | 75.5 | 1.95e12 | 2.47e9 | 2492 | 30 | 1 | 15.0 | 455 |
| 1946 | 53 | 131 | 25 | 68.8 | 2.11e12 | 2.50e9 | 3651 | 40 | 1 | 18.9 | 820 |
| 1947 | 78 | 138 | 25 | 68.1 | 2.25e12 | 2.53e9 | 4522 | 46 | 1 | 21.6 | 1185 |
| 1948 | 99 | 143 | 21 | 57.8 | 2.39e12 | 2.56e9 | 5194 | 56 | 1 | 38.3 | 1551 |
| 1949 | 120 | 146 | 22 | 59.6 | 2.53e12 | 2.59e9 | 5792 | 63 | 1 | 31.0 | 1916 |
| 1950 | 141 | 148 | 21 | 57.2 | 2.67e12 | 2.61e9 | 6347 | 74 | 1 | 23.7 | 2281 |
| 1951 | 160 | 149 | 19 | 51.8 | 2.82e12 | 2.64e9 | 6785 | 80 | 1 | 16.4 | 2646 |
| 1952 | 182 | 151 | 22 | 59.5 | 2.96e12 | 2.67e9 | 7121 | 90 | 2 | 15.0 | 3012 |
| 1953 | 205 | 153 | 23 | 63.9 | 3.12e12 | 2.70e9 | 7480 | 98 | 2 | 15.0 | 3377 |
| 1954 | 231 | 154 | 26 | 70.8 | 3.27e12 | 2.73e9 | 7788 | 105 | 2 | 15.0 | 3742 |
| 1955 | 254 | 155 | 23 | 63.3 | 3.44e12 | 2.77e9 | 8122 | 118 | 2 | 18.9 | 4107 |
| 1956 | 282 | 156 | 28 | 75.9 | 3.60e12 | 2.80e9 | 8499 | 128 | 2 | 15.0 | 4473 |
| 1957 | 310 | 157 | 28 | 77.4 | 3.77e12 | 2.83e9 | 8861 | 135 | 2 | 15.0 | 4838 |
| 1958 | 340 | 156 | 30 | 82.1 | 3.94e12 | 2.86e9 | 9217 | 144 | 2 | 15.0 | 5203 |
| 1959 | 370 | 157 | 30 | 81.2 | 4.12e12 | 2.89e9 | 9546 | 149 | 2 | 15.0 | 5568 |
| 1960 | 399 | 159 | 29 | 79.8 | 4.29e12 | 2.92e9 | 9865 | 174 | 2 | 15.0 | 5934 |
| 1961 | 428 | 159 | 29 | 79.2 | 4.47e12 | 2.96e9 | 10186 | 189 | 2 | 15.0 | 6299 |
| 1962 | 451 | 160 | 23 | 63.0 | 4.64e12 | 2.99e9 | 10424 | 198 | 2 | 15.0 | 6664 |
| 1963 | 473 | 160 | 22 | 61.0 | 4.84e12 | 3.02e9 | 10732 | 202 | 2 | 15.0 | 7029 |
| 1964 | 496 | 161 | 23 | 62.7 | 5.04e12 | 3.06e9 | 10961 | 206 | 2 | 15.0 | 7390 |

## PIB por país (nominal/ano)

| Ano | Mundo | Cresc. | USA | SOV | GBR | FRA | GER | JAP | CHI | IND | BRA |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1945 | 1.948e12 | - | 3.07e11 | 2.15e11 | 9.13e10 | 5.64e10 | 1.16e11 | 8.46e10 | 1.35e11 | 2.16e11 | 2.99e10 |
| 1946 | 2.114e12 | +8.5% | 3.22e11 | 2.28e11 | 1.01e11 | 6.13e10 | 1.23e11 | 9.07e10 | 1.42e11 | 2.30e11 | 3.40e10 |
| 1947 | 2.254e12 | +6.6% | 3.36e11 | 2.40e11 | 1.08e11 | 6.58e10 | 1.26e11 | 9.58e10 | 1.47e11 | 2.40e11 | 3.70e10 |
| 1948 | 2.394e12 | +6.2% | 3.49e11 | 2.52e11 | 1.15e11 | 7.05e10 | 1.31e11 | 9.95e10 | 1.57e11 | 2.50e11 | 3.92e10 |
| 1949 | 2.531e12 | +5.7% | 3.62e11 | 2.65e11 | 1.22e11 | 7.44e10 | 1.33e11 | 1.02e11 | 1.63e11 | 2.61e11 | 4.20e10 |
| 1950 | 2.675e12 | +5.7% | 3.76e11 | 2.76e11 | 1.28e11 | 7.98e10 | 1.37e11 | 1.07e11 | 1.68e11 | 2.71e11 | 4.54e10 |
| 1951 | 2.818e12 | +5.4% | 3.90e11 | 2.89e11 | 1.35e11 | 8.41e10 | 1.40e11 | 1.10e11 | 1.73e11 | 2.82e11 | 4.78e10 |
| 1952 | 2.963e12 | +5.1% | 4.05e11 | 3.01e11 | 1.40e11 | 8.85e10 | 1.45e11 | 1.15e11 | 1.78e11 | 2.93e11 | 5.08e10 |
| 1953 | 3.120e12 | +5.3% | 4.23e11 | 3.13e11 | 1.48e11 | 9.34e10 | 1.48e11 | 1.21e11 | 1.84e11 | 3.10e11 | 5.41e10 |
| 1954 | 3.265e12 | +4.7% | 4.38e11 | 3.28e11 | 1.53e11 | 9.84e10 | 1.52e11 | 1.24e11 | 1.93e11 | 3.18e11 | 5.76e10 |
| 1955 | 3.438e12 | +5.3% | 4.56e11 | 3.42e11 | 1.60e11 | 1.04e11 | 1.57e11 | 1.29e11 | 2.01e11 | 3.35e11 | 6.08e10 |
| 1956 | 3.602e12 | +4.8% | 4.72e11 | 3.54e11 | 1.67e11 | 1.09e11 | 1.61e11 | 1.34e11 | 2.08e11 | 3.47e11 | 6.39e10 |
| 1957 | 3.771e12 | +4.7% | 4.87e11 | 3.67e11 | 1.73e11 | 1.13e11 | 1.65e11 | 1.38e11 | 2.17e11 | 3.62e11 | 6.73e10 |
| 1958 | 3.942e12 | +4.5% | 5.03e11 | 3.79e11 | 1.80e11 | 1.18e11 | 1.71e11 | 1.42e11 | 2.26e11 | 3.77e11 | 7.08e10 |
| 1959 | 4.119e12 | +4.5% | 5.18e11 | 3.91e11 | 1.85e11 | 1.24e11 | 1.76e11 | 1.47e11 | 2.36e11 | 3.91e11 | 7.46e10 |
| 1960 | 4.287e12 | +4.1% | 5.34e11 | 3.99e11 | 1.91e11 | 1.30e11 | 1.80e11 | 1.50e11 | 2.45e11 | 4.09e11 | 7.81e10 |
| 1961 | 4.468e12 | +4.2% | 5.55e11 | 4.06e11 | 1.98e11 | 1.34e11 | 1.86e11 | 1.59e11 | 2.53e11 | 4.22e11 | 8.20e10 |
| 1962 | 4.644e12 | +3.9% | 5.72e11 | 4.17e11 | 2.06e11 | 1.40e11 | 1.90e11 | 1.64e11 | 2.60e11 | 4.31e11 | 8.60e10 |
| 1963 | 4.838e12 | +4.2% | 5.89e11 | 4.30e11 | 2.14e11 | 1.45e11 | 1.93e11 | 1.70e11 | 2.71e11 | 4.43e11 | 9.03e10 |
| 1964 | 5.036e12 | +4.1% | 6.06e11 | 4.42e11 | 2.21e11 | 1.50e11 | 1.96e11 | 1.75e11 | 2.81e11 | 4.75e11 | 9.36e10 |

## Tamanho máximo das estruturas (explosão de entidades)

- eventos_disparados: 150
- filas_construcao(soma): 2335
- jornal_eventos: 206
- log_diplomacia: 169
- pactos_diplomacia: 2
- paises_na_economia: 155
- pendentes_eventos: 0
- pops_max_por_provincia: 132
- rotas_comerciais: 20
- tags_legado: 174

## Pânicos

Nenhum pânico.

## Achados (NaN, valores absurdos, auditorias)

Nenhum.

## Descolonização e formáveis

| Data | Tag | Nome | Origens | Províncias | ADM1 | População |
|---|---|---|---|---|---|---|
| 1945-04-27 | AUT | Áustria | ["GER"] | 9 | 9 | 6800000 |
| 1945-08-17 | IDN | Indonésia | ["INS"] | 70 | 70 | 72000000 |
| 1945-09-02 | VDR | Vietnã do Norte (RDV) | ["FIC"] | 29 | 29 | 6792032 |
| 1946-01-01 | FIC | Indochina | [] | 75 | 75 | 21207968 |
| 1946-01-01 | IRN | Irã | [] | 59 | 59 | 15000001 |
| 1946-03-22 | JOR | Transjordânia | ["GBR"] | 12 | 0 | 400001 |
| 1946-04-05 | DEN | Bornholm | [] | 1 | 1 | 4050001 |
| 1946-05-01 | CHI | Manchúria | [] | 24 | 24 | 520000000 |
| 1946-07-04 | PHI | Filipinas | ["USA"] | 118 | 0 | 18099999 |
| 1947-08-14 | PAK | Paquistão | ["IND"] | 28 | 28 | 83894506 |
| 1947-08-15 | IND | Índia Britânica (Raj) | ["GBR"] | 109 | 0 | 306105495 |
| 1948-01-04 | BUR | Birmânia Britânica | ["GBR"] | 23 | 0 | 16999999 |
| 1948-02-04 | CEY | Ceilão Britânico | ["GBR"] | 25 | 0 | 6600001 |
| 1948-05-14 | ISR | Israel | ["PAL"] | 6 | 6 | 1324867 |
| 1948-05-14 | PAL | Mandato da Palestina | ["GBR"] | 2 | 0 | 425133 |
| 1948-08-15 | ROK | Coreia do Sul | ["KOR"] | 17 | 17 | 12454311 |
| 1948-09-09 | PRK | Coreia do Norte (RPDC) | ["KOR"] | 11 | 11 | 13045689 |
| 1949-05-23 | FRG | Alemanha Ocidental (RFA) | ["GER"] | 12 | 12 | 44087565 |
| 1949-10-01 | PRC | República Popular da China | ["CHI", "CCP"] | 275 | 275 | 511696240 |
| 1949-10-07 | GDR | Alemanha Oriental (RDA) | ["GER"] | 6 | 6 | 21412435 |
| 1951-12-24 | LBY | Líbia (Admin. Britânica) | ["GBR"] | 58 | 0 | 1000000 |
| 1952-04-28 | JAP | Japão | [] | 48 | 48 | 72150001 |
| 1953-10-22 | LAO | Laos | ["FIC"] | 17 | 17 | 6669355 |
| 1953-11-09 | KHM | Camboja | ["FIC"] | 24 | 24 | 6651268 |
| 1955-10-26 | RVN | Vietnã do Sul (RVN) | ["FIC"] | 34 | 34 | 7887345 |
| 1956-01-01 | SUD | Sudão Anglo-Egípcio | ["GBR"] | 80 | 0 | 7500000 |
| 1956-03-02 | MOR | Protetorado Francês de Marrocos | ["FRA"] | 26 | 0 | 8300001 |
| 1956-03-20 | TUN | Protetorado Francês da Tunísia | ["FRA"] | 23 | 0 | 3200001 |
| 1957-03-06 | GHA | Costa do Ouro Britânica | ["GBR"] | 11 | 0 | 4200001 |
| 1957-08-31 | MLY | Federação da Malaia | ["MAL"] | 21 | 21 | 6028833 |
| 1958-02-01 | UAR | República Árabe Unida | ["EGY", "SYR"] | 48 | 48 | 18500000 |
| 1958-10-02 | GIN | Guiné | ["AOF"] | 34 | 34 | 1630516 |
| 1960-01-01 | CMR | Camarões Francês | ["FRA"] | 17 | 0 | 4200000 |
| 1960-04-27 | TGO | Togolândia Francesa | ["FRA"] | 5 | 0 | 1250000 |
| 1960-06-26 | MDG | Madagascar Francês | ["FRA"] | 28 | 0 | 4000000 |
| 1960-06-30 | CGO | Congo Belga | ["BEL"] | 70 | 0 | 12000000 |
| 1960-07-01 | SMA | Somália | ["ISO", "SOM"] | 19 | 19 | 1592388 |
| 1960-07-11 | KAT | Katanga | ["CGO"] | 70 | 70 | 12000000 |
| 1960-08-01 | BEN | Daomé (Benim) | ["AOF"] | 12 | 12 | 610116 |
| 1960-08-03 | NER | Níger | ["AOF"] | 37 | 37 | 3845751 |
| 1960-08-05 | BFA | Alto Volta | ["AOF"] | 45 | 45 | 2029791 |
| 1960-08-07 | CIV | Costa do Marfim | ["AOF"] | 19 | 19 | 1418527 |
| 1960-08-11 | TCD | Chade | ["AEF"] | 45 | 45 | 2457181 |
| 1960-08-13 | CAF | República Centro-Africana | ["AEF"] | 25 | 25 | 1288702 |
| 1960-08-15 | COG | Congo-Brazzaville | ["AEF"] | 15 | 15 | 715249 |
| 1960-08-16 | CYP | Chipre Britânico | ["GBR"] | 8 | 0 | 449999 |
| 1960-08-17 | GAB | Gabão | ["AEF"] | 10 | 10 | 538868 |
| 1960-08-20 | SEN | Senegal | ["AOF"] | 14 | 14 | 912674 |
| 1960-09-22 | MLI | Mali | ["AOF"] | 40 | 40 | 4060319 |
| 1960-10-01 | NGA | Nigéria Britânica | ["GBR"] | 43 | 0 | 28000000 |
| 1960-11-28 | MRT | Mauritânia | ["AOF"] | 36 | 36 | 3492306 |
| 1961-04-27 | SLE | Serra Leoa Britânica | ["GBR"] | 4 | 0 | 1900000 |
| 1961-06-19 | KWT | Kuwait | ["GBR"] | 6 | 0 | 119999 |
| 1961-12-09 | TZA | Tanganica Britânica | ["GBR"] | 41 | 0 | 7300000 |
| 1962-07-01 | RWA | Ruanda | ["RUA"] | 5 | 5 | 1378170 |
| 1962-07-01 | BDI | Burundi | ["RUA"] | 17 | 17 | 2721830 |
| 1962-07-05 | DZA | Argélia Francesa | ["FRA"] | 99 | 0 | 8499999 |
| 1962-08-06 | JAM | Jamaica | ["BWI"] | 14 | 14 | 506196 |
| 1962-08-31 | TTO | Trinidad e Tobago | ["BWI"] | 16 | 16 | 360420 |
| 1962-10-09 | UGA | Uganda Britânico | ["GBR"] | 112 | 0 | 4700000 |
| 1963-09-16 | MYS | Malásia | ["MLY", "SWK", "NBO"] | 21 | 21 | 6028833 |
| 1963-12-12 | KEN | Quênia Britânico | ["GBR"] | 22 | 0 | 5500000 |
| 1964-04-26 | TNZ | Tanzânia | ["TZA", "ZAN"] | 41 | 41 | 7300000 |
| 1964-07-06 | MWI | Maláui | ["NYA"] | 28 | 28 | 2700000 |
| 1964-09-21 | MLT | Malta Britânica | ["GBR"] | 68 | 0 | 279999 |
| 1964-10-24 | ZMB | Zâmbia | ["NRH"] | 25 | 25 | 2200000 |

## Cobertura de eventos

- catálogo: 382 eventos; únicos disparados: 150; pendências resolvidas: 76.
- repetidos: 0; alvos inexistentes: 0; referências quebradas: 0; condições sem suporte: 0 chaves.
