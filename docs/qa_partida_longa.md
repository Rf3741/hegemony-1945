# QA: partida longa headless

Gerado por `cargo run --example partida_longa` (crate guerra_fria). Perfil dev (opt-level 1 no workspace, 3 nas dependências, overflow checks ligados).

- Período: 1945-01-01 + 20 anos (7300 dias); IA ligada: tecnologia (`escolher_ia` para todos), eventos (jogador USA tratado como IA via `escolha_ia`), diplomacia (`tick_dia`), política, economia + legado; descolonização/formações (`Formacoes::processar`, integradas em economia, política e diplomacia como no cliente).
- Carga: 4.0s (inclui 90 dias de aquecimento da economia). Tempo total da simulação: 625s.
- RSS: inicial 4 MB, após carga 152 MB, final 152 MB.
- Eventos de IA resolvidos pelo "jogador": 72.

## Tempo médio por dia (ms)

| Sistema | ms/dia |
|---|---|
| economia (sim_economia) | 69.95 |
| legado (simulacao::executar_tick_economia) | 2.33 |
| formações (descolonização) | 0.06 |
| política | 9.13 |
| diplomacia | 0.02 |
| tecnologia | 1.62 |
| eventos | 0.08 |

## Resumo anual

| Ano | t(s) | RSS MB | s/ano | ms/dia | PIB mundial/ano | pop. mundial | techs conhecidas | notícias | pactos | tensão | dias eco |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1945 | 21 | 122 | 21 | 57.2 | 1.95e12 | 2.47e9 | 2492 | 28 | 1 | 15.0 | 455 |
| 1946 | 44 | 131 | 23 | 63.4 | 2.11e12 | 2.50e9 | 3651 | 38 | 1 | 18.9 | 820 |
| 1947 | 69 | 138 | 25 | 67.5 | 2.25e12 | 2.53e9 | 4522 | 44 | 1 | 21.6 | 1185 |
| 1948 | 96 | 143 | 27 | 73.3 | 2.39e12 | 2.56e9 | 5193 | 54 | 1 | 38.3 | 1551 |
| 1949 | 124 | 146 | 28 | 76.8 | 2.53e12 | 2.59e9 | 5792 | 61 | 1 | 31.0 | 1916 |
| 1950 | 150 | 148 | 27 | 73.6 | 2.68e12 | 2.61e9 | 6345 | 70 | 1 | 23.7 | 2281 |
| 1951 | 178 | 150 | 28 | 76.2 | 2.82e12 | 2.64e9 | 6785 | 75 | 1 | 16.4 | 2646 |
| 1952 | 207 | 152 | 29 | 78.3 | 2.96e12 | 2.67e9 | 7121 | 82 | 2 | 15.0 | 3012 |
| 1953 | 235 | 153 | 28 | 77.3 | 3.12e12 | 2.70e9 | 7479 | 88 | 2 | 15.0 | 3377 |
| 1954 | 263 | 154 | 27 | 75.1 | 3.27e12 | 2.73e9 | 7787 | 93 | 2 | 15.0 | 3742 |
| 1955 | 292 | 147 | 30 | 81.6 | 3.43e12 | 2.77e9 | 8121 | 105 | 2 | 18.9 | 4107 |
| 1956 | 326 | 149 | 33 | 91.3 | 3.61e12 | 2.80e9 | 8498 | 113 | 2 | 15.0 | 4473 |
| 1957 | 364 | 150 | 38 | 104.2 | 3.77e12 | 2.83e9 | 8859 | 118 | 2 | 15.0 | 4838 |
| 1958 | 402 | 147 | 38 | 104.9 | 3.94e12 | 2.86e9 | 9217 | 126 | 2 | 15.0 | 5203 |
| 1959 | 440 | 148 | 38 | 104.5 | 4.12e12 | 2.89e9 | 9545 | 131 | 2 | 15.0 | 5568 |
| 1960 | 479 | 149 | 39 | 106.5 | 4.28e12 | 2.92e9 | 9867 | 155 | 2 | 15.0 | 5934 |
| 1961 | 517 | 150 | 38 | 104.1 | 4.46e12 | 2.96e9 | 10189 | 169 | 2 | 15.0 | 6299 |
| 1962 | 556 | 151 | 39 | 106.8 | 4.65e12 | 2.99e9 | 10423 | 178 | 2 | 15.0 | 6664 |
| 1963 | 596 | 151 | 40 | 109.6 | 4.84e12 | 3.02e9 | 10733 | 185 | 2 | 15.0 | 7029 |
| 1964 | 625 | 151 | 29 | 80.5 | 5.03e12 | 3.06e9 | 10960 | 194 | 2 | 15.0 | 7390 |

## PIB por país (nominal/ano)

| Ano | Mundo | Cresc. | USA | SOV | GBR | FRA | GER | JAP | CHI | IND | BRA |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1945 | 1.948e12 | - | 3.07e11 | 2.15e11 | 9.13e10 | 5.64e10 | 1.16e11 | 8.46e10 | 1.35e11 | 2.16e11 | 2.99e10 |
| 1946 | 2.114e12 | +8.5% | 3.22e11 | 2.28e11 | 1.01e11 | 6.13e10 | 1.23e11 | 9.07e10 | 1.42e11 | 2.30e11 | 3.40e10 |
| 1947 | 2.253e12 | +6.6% | 3.36e11 | 2.40e11 | 1.08e11 | 6.58e10 | 1.26e11 | 9.58e10 | 1.47e11 | 2.40e11 | 3.70e10 |
| 1948 | 2.394e12 | +6.2% | 3.49e11 | 2.52e11 | 1.15e11 | 7.05e10 | 1.31e11 | 9.93e10 | 1.57e11 | 2.50e11 | 3.92e10 |
| 1949 | 2.533e12 | +5.8% | 3.62e11 | 2.66e11 | 1.22e11 | 7.45e10 | 1.33e11 | 1.03e11 | 1.63e11 | 2.61e11 | 4.20e10 |
| 1950 | 2.678e12 | +5.7% | 3.76e11 | 2.77e11 | 1.28e11 | 7.98e10 | 1.37e11 | 1.06e11 | 1.68e11 | 2.73e11 | 4.54e10 |
| 1951 | 2.818e12 | +5.2% | 3.90e11 | 2.90e11 | 1.35e11 | 8.40e10 | 1.40e11 | 1.10e11 | 1.73e11 | 2.81e11 | 4.82e10 |
| 1952 | 2.963e12 | +5.2% | 4.05e11 | 3.02e11 | 1.40e11 | 8.84e10 | 1.45e11 | 1.16e11 | 1.78e11 | 2.93e11 | 5.11e10 |
| 1953 | 3.120e12 | +5.3% | 4.23e11 | 3.14e11 | 1.47e11 | 9.34e10 | 1.48e11 | 1.20e11 | 1.84e11 | 3.12e11 | 5.42e10 |
| 1954 | 3.269e12 | +4.8% | 4.38e11 | 3.28e11 | 1.53e11 | 9.84e10 | 1.52e11 | 1.25e11 | 1.93e11 | 3.21e11 | 5.75e10 |
| 1955 | 3.434e12 | +5.0% | 4.56e11 | 3.42e11 | 1.60e11 | 1.04e11 | 1.57e11 | 1.29e11 | 2.01e11 | 3.33e11 | 6.06e10 |
| 1956 | 3.606e12 | +5.0% | 4.72e11 | 3.54e11 | 1.67e11 | 1.09e11 | 1.61e11 | 1.33e11 | 2.09e11 | 3.51e11 | 6.39e10 |
| 1957 | 3.772e12 | +4.6% | 4.87e11 | 3.68e11 | 1.73e11 | 1.13e11 | 1.66e11 | 1.37e11 | 2.18e11 | 3.61e11 | 6.73e10 |
| 1958 | 3.938e12 | +4.4% | 5.03e11 | 3.79e11 | 1.80e11 | 1.18e11 | 1.71e11 | 1.41e11 | 2.27e11 | 3.74e11 | 7.09e10 |
| 1959 | 4.122e12 | +4.7% | 5.19e11 | 3.90e11 | 1.85e11 | 1.24e11 | 1.76e11 | 1.48e11 | 2.37e11 | 3.95e11 | 7.50e10 |
| 1960 | 4.282e12 | +3.9% | 5.34e11 | 3.97e11 | 1.91e11 | 1.30e11 | 1.80e11 | 1.53e11 | 2.44e11 | 4.06e11 | 7.80e10 |
| 1961 | 4.462e12 | +4.2% | 5.55e11 | 4.06e11 | 1.98e11 | 1.34e11 | 1.86e11 | 1.59e11 | 2.53e11 | 4.19e11 | 8.26e10 |
| 1962 | 4.648e12 | +4.2% | 5.72e11 | 4.15e11 | 2.07e11 | 1.40e11 | 1.90e11 | 1.64e11 | 2.61e11 | 4.34e11 | 8.56e10 |
| 1963 | 4.839e12 | +4.1% | 5.89e11 | 4.27e11 | 2.17e11 | 1.45e11 | 1.93e11 | 1.70e11 | 2.71e11 | 4.46e11 | 9.06e10 |
| 1964 | 5.030e12 | +3.9% | 6.05e11 | 4.39e11 | 2.24e11 | 1.50e11 | 1.96e11 | 1.75e11 | 2.81e11 | 4.70e11 | 9.36e10 |

## Tamanho máximo das estruturas (explosão de entidades)

- eventos_disparados: 128
- filas_construcao(soma): 2335
- jornal_eventos: 194
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

- 1947-08-18 economia: tesouro de cgo saltou de 88198 para 534666 em 30 dias

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

- catálogo: 382 eventos; únicos disparados: 128; pendências resolvidas: 72.
- repetidos: 0; alvos inexistentes: 3; referências quebradas: 1; condições sem suporte: 196 chaves.

### Eventos históricos que nunca dispararam na janela (20 items)

- truman_bomba_hidrogenio_1950 [1950-01-31] alvos ["USA", "SOV"]
- diretriz_nsc68_militarizacao_1950 [1950-04-14] alvos ["USA"]
- retorno_winston_churchill_1951 [1951-10-26] alvos ["GBR"]
- sucessao_rainha_elizabeth_ii_1952 [1952-02-06] alvos ["GBR"]
- reforma_agraria_arbenz_guatemala_1952 [1952-06-17] alvos ["GUA", "USA"]
- emergencia_mau_mau_quenia_1952 [1952-10-20] alvos ["KEN", "GBR"]
- morte_de_stalin_luta_sucessoria_1953 [1953-03-05] alvos ["SOV"]
- revolta_operaria_berlim_oriental_1953 [1953-06-17] alvos ["GDR", "SOV", "FRG", "USA"]
- operacao_pbsuccess_guatemala_1954 [1954-06-27] alvos ["GUA", "USA"]
- eclosao_guerra_argelia_toussaint_1954 [1954-11-01] alvos ["DZA", "FRA"]
- criacao_pacto_varsovia_1955 [1955-05-14] alvos ["SOV", "POL", "CZE", "GDR", "HUN", "ROM", "BUL", "ALB"]
- discurso_secreto_khrushchev_1956 [1956-02-25] alvos ["SOV"]
- revolta_operaria_poznan_1956 [1956-06-28] alvos ["POL", "SOV"]
- expulsoes_grupo_antipartido_urss_1957 [1957-06-29] alvos ["SOV"]
- lancamento_sputnik_1_1957 [1957-10-04] alvos ["SOV", "USA"]
- grande_salto_adiante_china_1958 [1958-05-16] alvos ["PRC"]
- eleicao_john_f_kennedy_1960 [1960-11-08] alvos ["USA"]
- voo_yuri_gagarin_vostok1_1961 [1961-04-12] alvos ["SOV", "USA"]
- assassinato_de_john_f_kennedy_1963 [1963-11-22] alvos ["USA"]
- deponimento_khrushchev_brezhnev_1964 [1964-10-14] alvos ["SOV"]

### Eventos com alvo inexistente no mapa (3 items)

- golpe_militar_uruguai_1973 [1973-06-27] alvos ["URU"] não existem no mapa
- massacre_politecnica_atenas_1973 [1973-11-14] alvos ["GRE"] não existem no mapa
- revolucao_romena_execucao_ceausescu_1989 [1989-12-25] alvos ["ROM"] não existem no mapa

### Referências pos_evento quebradas (1 items)

- truman_bomba_hidrogenio_1950 aponta pos_evento:primeiro_teste_atomico_sovietico_1949 (id inexistente)

### Condições que o cliente não sabe avaliar (viram `true`)

- abertura_cortina_de_ferro (x1)
- abertura_socialista_em_curso (x1)
- acordo_paz_afeganistao (x1)
- alinhamento:bloco_ocidental (x6)
- ameaca_intervencao_pacto_varsovia (x1)
- anos_de_chumbo_italia (x1)
- apartheid_institucional (x1)
- bloco_sovietico (x3)
- boicote_olimpico_guerra_fria (x1)
- canal_suez_estatizado (x1)
- catastrofe_humanitaria_africa (x1)
- catastrofe_nuclear_sovietica (x1)
- cisao_sino_sovietica (x1)
- colapso_ditadura_ceausescu (x1)
- combate_ao_terrorismo_estatal (x1)
- conclusao_retirada_militar (x1)
- conflito_africa_austral (x1)
- conflito_arabe_israelense (x2)
- conflito_chifre_africa (x1)
- conflito_etnico_caucaso (x1)
- conflito_golfo_persico (x1)
- conflito_ideologico_religiao (x1)
- conflito_libanes_massacre (x1)
- conflito_libano_guerra_civil (x1)
- conflito_oriente_medio (x2)
- conflito_sindical_carvao (x1)
- conflito_soberania_atlantico_sul (x1)
- conflito_sul_asiatico (x1)
- conflito_terceira_indochina (x1)
- conflito_trabalhista_neoliberal (x1)
- confronto_eua_ira (x1)
- congresso_partidario_reformas (x1)
- corrida_armamentista_nuclear (x2)
- corrida_tecnologica_militar (x1)
- crise_abastecimento_polonia (x1)
- crise_bloco_sovietico (x1)
- crise_comunal_punjab (x1)
- crise_constitucional_watergate (x1)
- crise_divida_terceiro_mundo (x1)
- crise_energetica (x1)
- crise_energetica_global (x1)
- crise_estagflacao_anos_70 (x1)
- crise_financeira_internacional (x1)
- crise_golfo_1990 (x1)
- crise_governanca_cia (x1)
- crise_irlanda_norte (x1)
- crise_modelo_social_democrata (x1)
- crise_otan_mediterraneo (x1)
- crise_politica_chile (x1)
- crise_refens_teera (x1)
- crise_sucessao_comunista (x1)
- crise_terminal_urss (x1)
- crise_terrorismo_refens (x1)
- crise_trabalhista_polonia (x1)
- crise_vaticana_guerra_fria (x1)
- cuba_sob_ditadura_batista (x1)
- cupula_desarmamento_reykjavik (x1)
- cupula_moscou_start (x1)
- cupula_superpotencias (x1)
- cupula_washington_inf (x1)
- democracia_presidencialista (x4)
- democratizacao_tigres_asiaticos (x1)
- descolonizacao_africana (x2)
- descolonizacao_negociada (x1)
- descolonizacao_sudeste_asiatico (x1)
- desescalada_militar_global (x1)
- desestalinizacao_em_curso (x1)
- desmantelamento_bloco_leste (x1)
- dissidencia_intelectual (x1)
- dissolucao_territorial_urss (x1)
- doutrina_brejnev (x1)
- doutrina_defesa_golfo (x1)
- doutrina_reagan_ativa (x1)
- doutrina_seguranca_nacional (x2)
- doutrina_sinatra_renuncia_forca (x1)
- dubcek_lideranca_tcheca (x1)
- egito_republicano_nasser (x1)
- eleicoes_democracia_latam (x1)
- era_destensao (x4)
- estagnacao_brejneviana (x1)
- estagnacao_economica_brejneviana (x1)
- expansao_universitaria_pos_guerra (x1)
- fim_apartheid_transicao (x1)
- fim_autoritarismo_colonial (x1)
- fim_conflito_golfo_persico (x1)
- fim_crise_refens (x1)
- fim_da_detente (x1)
- fim_da_urss (x1)
- fim_ditadura_cone_sul (x1)
- fim_ditadura_pinochet (x1)
- fim_gerontocracia (x1)
- fim_oficial_guerra_fria (x1)
- fim_regime_franquista (x1)
- fim_regime_minoria_branca (x1)
- gerontocracia_sovietica (x1)
- golpe_comunista_asia (x1)
- gorbachev_secretario_geral (x1)
- guerra_do_golfo_ativa (x1)
- guerra_em_curso:guerra_da_coreia (x3)
- guerra_na_europa_em_andamento (x1)
- guerra_precos_petroleo (x1)
- guerra_proxy (x1)
- guerra_terceira_indochina (x1)
- guerra_vietna_ativa (x1)
- guerra_yom_kippur_ou_tensao_opep (x1)
- guerrilha_sierra_maestra_ativa (x1)
- hungria_no_pacto_de_varsovia (x1)
- insatisfacao_trabalhadores (x1)
- instabilidade_pos_colonial (x1)
- integracao_europeia (x1)
- integracao_europeia_avancada (x1)
- intervencao_multinacional_libano (x1)
- intervencionismo_caribenho (x1)
- intervencionismo_pos_guerra_fria (x1)
- ira_sob_dinastia_pahlavi (x1)
- khrushchev_primeiro_secretario (x1)
- lider_ativo:carlos_castillo_armas (x1)
- lider_ativo:charles_de_gaulle (x5)
- lider_ativo:chiang_kai_shek (x1)
- lider_ativo:douglas_macarthur (x2)
- lider_ativo:dwight_d_eisenhower (x4)
- lider_ativo:fidel_castro (x4)
- lider_ativo:fulgencio_batista (x3)
- lider_ativo:gamal_abdel_nasser (x3)
- lider_ativo:getulio_vargas (x2)
- lider_ativo:harry_s_truman (x2)
- lider_ativo:ho_chi_minh (x1)
- lider_ativo:imre_nagy (x1)
- lider_ativo:iosif_stalin (x1)
- lider_ativo:jacobo_arbenz (x2)
- lider_ativo:jawaharlal_nehru (x2)
- lider_ativo:joao_goulart (x3)
- lider_ativo:josip_broz_tito (x1)
- lider_ativo:juscelino_kubitschek (x1)
- lider_ativo:kim_il_sung (x1)
- lider_ativo:konrad_adenauer (x1)
- lider_ativo:mao_zedong (x6)
- lider_ativo:mohammad_mossadegh (x2)
- lider_ativo:mohammad_reza_pahlavi (x1)
- lider_ativo:ngo_dinh_diem (x2)
- lider_ativo:nikita_khrushchev (x7)
- lider_ativo:sukarno (x2)
- lider_ativo:syngman_rhee (x1)
- lider_ativo:vo_nguyen_giap (x1)
- lider_ativo:winston_churchill (x1)
- lider_ativo:wladyslaw_gomulka (x1)
- lider_ativo:zhou_enlai (x1)
- multilateralismo_onu (x1)
- muro_de_berlim_ativo (x1)
- negociacao_oriente_medio (x1)
- nova_ordem_seguranca_europeia (x1)
- ocupacao_territorios_palestinos (x1)
- ofensiva_primavera_vietna (x1)
- onda_independencias_africanas (x1)
- ostpolitik_ativa (x1)
- paises_independentes_asia_africa_presentes (x1)
- papa_joao_paulo_ii_eleito (x1)
- paranoia_militar_espaco_aereo (x1)
- paranoia_nuclear_segunda_guerra_fria (x1)
- parlamentarismo_europeu (x1)
- pontificado_joao_xxiii_ou_paulo_vi (x1)
- pos_revolucao_cultural (x1)
- proliferacao_nuclear (x1)
- proliferacao_nuclear_oriente_medio (x1)
- prosperidade_economica_eleicao (x1)
- protestos_em_massa_alemanha_oriental (x1)
- protestos_pro_democracia_china (x1)
- protestos_religiosos_urbanos (x1)
- queda_ditadura_pro_ocidental (x1)
- radicalismo_agrario_cambodiano (x1)
- radicalismo_islamico_oriente_medio (x1)
- regime_militar (x1)
- revolucao_cultural_tardia (x1)
- revolucao_marxista_africa (x1)
- revolucao_marxista_latam (x1)
- revolucao_teocratica (x1)
- revolucoes_1989_inicio (x1)
- revolucoes_1989_leste (x1)
- segunda_guerra_fria (x1)
- seguranca_nuclear (x1)
- separatismo_baltico (x1)
- soberania_canal_panama (x1)
- soviete_supremo_em_sessao (x1)
- stalin_no_poder (x1)
- tensao_autoritaria_cone_sul (x1)
- tensao_geracional_jovem (x1)
- tensao_guerra_fria_latam (x1)
- terrorismo_aereo_internacional (x1)
- terrorismo_internacional (x1)
- terrorismo_urbano_extrema_esquerda (x1)
- transicao_democratica_latam (x1)
- tratado_dois_mais_quatro (x1)
- triangulacao_diplomatica (x2)
- vitoria_conservadora_eleitoral (x1)
- volatilidade_mercado_financeiro (x1)
- xx_congresso_pcus_realizado (x1)
