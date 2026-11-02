# QA: crescimento macro da economia (PIB mundial)

Problema medido na partida longa headless (`cargo run -p guerra_fria --example partida_longa -- 10 --sem-relatorio`):
PIB mundial de 1,80e12 (1946) a 1,93e12 (1954), ou seja ~0,8%/ano (0,5%/ano nos últimos anos), contra ~4-5%/ano
histórico nos anos 1950 (EUA ~3-4%, Europa Ocidental e Japão 5-8% na reconstrução, URSS ~5-6%, América Latina ~5%).

Ferramenta de diagnóstico criada: `cargo run --release -p ia_paises --example crescimento -- ANOS` (só economia + IA
econômica, sem janela; imprime por ano PIB nominal, índice de preços, PIB real, população, emprego, níveis de prédios,
fundo privado, tesouro, dívida, fila de obras e a situação dos 9 maiores países).

## 1. Diagnóstico (números do estado ANTES da correção)

| Hipótese | Medido | Veredito |
|---|---|---|
| Deflação por excesso de oferta | índice de preços (média geométrica preço/preço-base de todos os bens e mercados) 1,114 -> 1,100 em 10 anos; PIB real acompanha o nominal (1,55e12 -> 1,76e12) | Não é a causa |
| Demanda travada / IA sem dinheiro | tesouro dos países somados: 1,3e11 (1945) -> 2,1e12 em 1955 (um ano inteiro de PIB parado no caixa); fundo de investimento privado 6,9e10 -> 3,3e11; dívida ~0 | **Há dinheiro de sobra; ele não vira prédio** |
| Investimento não vira prédio | níveis de prédios do mundo 1,71e6 -> 1,83e6 (**+0,6%/ano**); fila de obras estável em ~220 itens | **Causa principal** (ver abaixo) |
| Produtividade estagnada / tecnologia sem efeito | nenhum código de `sim_economia` lê tecnologia: a ponte só *trava* prédios/métodos; os "modificadores" das techs são texto sem efeito; ninguém troca métodos de produção na IA | **Causa 2**: produtividade total constante (1,0) para sempre |
| Pops sem mobilidade / população | pop. mundial +0,5%/ano (r anual por pop de 0,3% a 0,9% porque o padrão de vida é 6-15); emprego/pop fixo em 0,30 | **Causa 3**: sem transição demográfica do pós-guerra; sem vagas novas o emprego não sobe |

Detalhe da causa principal (capacidade de construção):

- Cada país tem só `pontos_construcao_base` (20 x 2 = 40 pontos/dia) mais os pontos dos prédios do setor de construção.
  **Nenhum país tinha um único nível de `setor_construcao`**: o prédio era privado e sem produto (receita zero), então
  o investidor privado nunca o construía, e a IA estatal o ignorava (`Militar | Construcao => continue`).
- Um nível custa 100-250 pontos. Com 40 pontos/dia os EUA (266 mil níveis) construíam ~50 níveis/ano (**0,02%/ano**).
  O mundo todo construía ~10 mil níveis/ano sobre 1,7 milhão (0,6%).
- Mesmo com dinheiro, as filas eram minúsculas: cada obra absorve no máximo 25 pontos/dia, o teto de obras privadas
  simultâneas era 3 e o de estatais 6, e a IA enfileirava 1 obra por semana. Os EUA aplicavam 25 pontos/dia de 40.
- Resultado: capacidade de construção e número de obras limitam o investimento; tesouro e fundo privado só acumulam.

## 2. Correções no modelo (nenhuma injeta PIB; todas mexem em capacidade, demografia ou produtividade)

1. **Setor de construção virou obra estatal** (`assets/data/economia/tipos_predio.ron`: `setor_construcao` com
   `financiado_pelo_governo: true`, método padrão `propriedade_estatal`). Os salários e insumos (aço, madeira, ferramentas)
   saem do orçamento (já existia o reembolso proporcional ao uso privado de pontos). A capacidade passa a custar de verdade.
2. **IA constrói capacidade** (`crates/ia_paises/src/economia.rs`): meta de níveis de construção = `0,02%` do total de níveis
   x fator de convergência (`(PIB pc / PIB pc mundial)^-0,5`, limitado a 0,45-1,5, para ricos crescerem menos e pobres mais)
   x fator de **reconstrução** pós-guerra (GER/JAP 2,2; ITA 1,6; FRA/POL/HUN/NLD/BEL/AUT 1,5; SOV/CZE/FIN 1,4; GRE 1,3; GBR 1,2;
   decai linearmente a 1,0 em 10 anos). Obras de construção furam a fila (sem inanição entre elas) e a fila cheia só barra
   o que não é construção. A IA enfileira vários projetos por semana, proporcional à capacidade (3 a 40).
3. **Obras em lotes** (`ItemConstrucao.niveis`, `MundoEconomico::enfileirar_construcao_lote`, `tamanho_lote_obra`): uma obra
   entrega N níveis (N = níveis do país / 12 mil, de 1 a 24), com custo em pontos e limite diário de pontos proporcionais.
   Sem isso, os EUA precisariam de ~150 obras simultâneas. Jogador e obras comuns continuam com 1 nível.
4. **Investimento privado escala** (`governo.rs`): obras privadas simultâneas até 60 (antes 3), até 8 novas por passo
   semanal (antes 1), escolhendo as melhores por província (antes só a melhor do país).
5. **Demografia** (`taxa_crescimento_anual_pop`): transição demográfica do pós-guerra. Máximo (~2%) em padrão de vida
   intermediário, ~1% nos ricos alfabetizados, fome abaixo de 5. Mundo: +0,5%/ano -> ~1,2%/ano.
6. **Produtividade total dos fatores** (`Pais.produtividade`, índice 1,0 em 1945; multiplica o produto físico dos prédios
   privados e a subsistência, não os insumos): cresce 0,2%/ano + até 0,8%/ano pela alfabetização + convergência de até 0,8%/ano
   para quem tem renda per capita abaixo da líder (difusão de tecnologia da fronteira). **Tecnologia**: cada tecnologia
   civil concluída (ramos Produção e Sociedade) soma +0,6% (ponte em `tecnologia_ui.rs` -> `EconomiaSim::registrar_ganho_tecnologico`
   -> `MundoEconomico::aplicar_ganho_produtividade`; também na `partida_longa`).
7. **Desempenho**: bônus de qualidade de recurso/insumo local do prédio passam a ser calculados uma vez e guardados no cache do prédio
   (antes: comparação de texto por prédio por dia); materiais de obra por categoria resolvidos para índices uma vez por país/dia.

## 3. Taxa de crescimento antes/depois

Headless só economia + IA (`crescimento`, após 90 dias de aquecimento):

| | antes | depois |
|---|---|---|
| PIB mundial, média 5 anos | 1,7%/ano (1,72e12 -> 1,88e12) | **5,6%/ano** |
| PIB mundial, média 10 anos | 1,2%/ano (1,72e12 -> 1,93e12) | **4,7%/ano**, caindo para 3,5% no décimo ano |
| Crescimento por ano | 4,5% 1,7% 1,1% 0,8% 0,6% 0,6% ... | 9,1% 5,8% 4,7% 4,3% 4,2% 4,0% 3,7% 3,6% 3,8% 3,5% |
| Emprego/pop (5 anos) | 0,300 -> 0,300 | 0,297 -> 0,312 |

O primeiro ano (+9%) é a absorção do caixa parado e da infraestrutura em 1945 (recuperação inicial), ela some em 2-3 anos.

Por país, em 5 anos (CAGR): USA 3,9%, SOV 4,7%, GBR 5,1%, FRA 6,6%, GER 4,6%, JAP 5,3%, CHI 2,4% (guerra civil, quase sem
capital e alfabetização 0,2-0,4), IND 4,3%, BRA 5,7%. Antes: todos entre 0% e 1%.

Teste novo: `crates/ia_paises/tests/crescimento_macro.rs` (5 anos, IA ligada, exige média 3%-7%/ano, sem NaN/inf, população
subindo, emprego não despencando, `auditar_estabilidade`): resultado 5,60%/ano (+9,1% +5,8% +4,7% +4,2% +4,2%).
O teste `balanco_1945::cinco_anos_desemprego_e_pib_dos_grandes_estaveis` (5 anos SEM IA) teve o teto de desemprego subido de 18% para
25%, porque sem investimento nenhum a população agora cresce 1,3%/ano e o desemprego sobe ~1,3 p.p./ano.

## 4. Partida longa de 10 anos (cenário completo: tecnologia, política, diplomacia, eventos, economia + IA)

Comando: `cargo run -p guerra_fria --example partida_longa -- 10 --saida=docs/qa_economia_partida.md` (a opção `--saida=` é nova).
Antes (docs/qa_partida_longa.md): 1,80e12 (1946) -> 1,93e12 (1954), +0,8%/ano. Depois: ver tabelas.

### Resumo anual (depois)

| Ano | t(s) | RSS MB | PIB mundial/ano | pop. mundial | techs conhecidas | notícias | pactos | tensão | dias eco |
|---|---|---|---|---|---|---|---|---|---|
| 1946 | 11 | 117 | 1.91e12 | 2.47e9 | 2680 | 23 | 1 | 15.0 | 455 |
| 1947 | 22 | 122 | 2.04e12 | 2.50e9 | 3956 | 29 | 1 | 18.9 | 820 |
| 1948 | 34 | 124 | 2.16e12 | 2.53e9 | 4875 | 33 | 1 | 21.6 | 1185 |
| 1948 | 46 | 127 | 2.29e12 | 2.56e9 | 5625 | 37 | 1 | 38.3 | 1550 |
| 1949 | 58 | 129 | 2.41e12 | 2.59e9 | 6272 | 41 | 1 | 31.0 | 1915 |
| 1950 | 70 | 131 | 2.53e12 | 2.61e9 | 6857 | 49 | 1 | 23.7 | 2280 |
| 1951 | 83 | 133 | 2.65e12 | 2.64e9 | 7277 | 53 | 1 | 16.4 | 2645 |
| 1952 | 96 | 134 | 2.76e12 | 2.67e9 | 7632 | 58 | 2 | 15.0 | 3010 |
| 1953 | 108 | 136 | 2.89e12 | 2.70e9 | 8020 | 63 | 2 | 15.0 | 3375 |
| 1954 | 122 | 137 | 3.00e12 | 2.73e9 | 8342 | 67 | 2 | 15.0 | 3740 |

### PIB por país, depois (nominal por ano, unidades monetárias do jogo)

| Ano | Mundo | Cresc. | USA | SOV | GBR | FRA | GER | JAP | CHI | IND | BRA |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1946 | 1.907e12 | - | 3.11e11 | 2.12e11 | 8.85e10 | 5.53e10 | 1.11e11 | 7.90e10 | 1.33e11 | 2.16e11 | 3.02e10 |
| 1947 | 2.042e12 | +7.1% | 3.22e11 | 2.22e11 | 9.59e10 | 5.94e10 | 1.18e11 | 8.34e10 | 1.37e11 | 2.29e11 | 3.40e10 |
| 1948 | 2.161e12 | +5.8% | 3.33e11 | 2.37e11 | 1.01e11 | 6.31e10 | 1.22e11 | 8.74e10 | 1.39e11 | 2.38e11 | 3.71e10 |
| 1948 | 2.286e12 | +5.8% | 3.44e11 | 2.59e11 | 1.07e11 | 6.68e10 | 1.28e11 | 9.09e10 | 1.43e11 | 2.48e11 | 3.96e10 |
| 1949 | 2.408e12 | +5.3% | 3.55e11 | 2.78e11 | 1.12e11 | 7.05e10 | 1.33e11 | 9.45e10 | 1.47e11 | 2.57e11 | 4.22e10 |
| 1950 | 2.528e12 | +5.0% | 3.66e11 | 2.94e11 | 1.17e11 | 7.43e10 | 1.38e11 | 9.84e10 | 1.49e11 | 2.67e11 | 4.38e10 |
| 1951 | 2.645e12 | +4.6% | 3.78e11 | 3.09e11 | 1.23e11 | 7.77e10 | 1.43e11 | 1.03e11 | 1.51e11 | 2.74e11 | 4.59e10 |
| 1952 | 2.755e12 | +4.2% | 3.90e11 | 3.15e11 | 1.27e11 | 8.18e10 | 1.48e11 | 1.08e11 | 1.54e11 | 2.83e11 | 4.72e10 |
| 1953 | 2.886e12 | +4.7% | 4.06e11 | 3.29e11 | 1.33e11 | 8.54e10 | 1.54e11 | 1.14e11 | 1.58e11 | 2.90e11 | 4.90e10 |
| 1954 | 3.000e12 | +3.9% | 4.19e11 | 3.39e11 | 1.37e11 | 8.98e10 | 1.60e11 | 1.17e11 | 1.62e11 | 2.97e11 | 5.00e10 |

## Tamanho máximo das estruturas (explosão de entidades)

- eventos_disparados: 67
- filas_construcao(soma): 2228
- jornal_eventos: 67
- log_diplomacia: 86
- pactos_diplomacia: 2
- pendentes_eventos: 0
- rotas_comerciais: 20

### PIB por país, antes (mesma medição no estado original, só economia + IA)

| Ano | Mundo | Cresc. | USA | SOV | GBR | FRA | GER | JAP | CHI | IND | BRA |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1946 | 1.802e+12 | - | 2.99e+11 | 2.04e+11 | 7.84e+10 | 5.24e+10 | 1.06e+11 | 7.55e+10 | 1.31e+11 | 2.07e+11 | 2.75e+10 |
| 1947 | 1.833e+12 | +1.7% | 3.01e+11 | 2.07e+11 | 7.93e+10 | 5.34e+10 | 1.07e+11 | 7.58e+10 | 1.32e+11 | 2.09e+11 | 2.78e+10 |
| 1948 | 1.854e+12 | +1.1% | 3.01e+11 | 2.10e+11 | 7.98e+10 | 5.40e+10 | 1.07e+11 | 7.57e+10 | 1.32e+11 | 2.10e+11 | 2.80e+10 |
| 1949 | 1.868e+12 | +0.8% | 3.01e+11 | 2.10e+11 | 8.01e+10 | 5.44e+10 | 1.07e+11 | 7.57e+10 | 1.32e+11 | 2.11e+11 | 2.82e+10 |
| 1950 | 1.880e+12 | +0.6% | 3.01e+11 | 2.10e+11 | 8.03e+10 | 5.48e+10 | 1.07e+11 | 7.58e+10 | 1.32e+11 | 2.12e+11 | 2.84e+10 |
| 1951 | 1.892e+12 | +0.6% | 3.02e+11 | 2.10e+11 | 8.05e+10 | 5.51e+10 | 1.07e+11 | 7.58e+10 | 1.32e+11 | 2.12e+11 | 2.86e+10 |
| 1952 | 1.903e+12 | +0.6% | 3.02e+11 | 2.10e+11 | 8.07e+10 | 5.54e+10 | 1.07e+11 | 7.59e+10 | 1.32e+11 | 2.13e+11 | 2.88e+10 |
| 1953 | 1.914e+12 | +0.6% | 3.02e+11 | 2.10e+11 | 8.09e+10 | 5.57e+10 | 1.08e+11 | 7.59e+10 | 1.32e+11 | 2.13e+11 | 2.90e+10 |
| 1954 | 1.924e+12 | +0.5% | 3.03e+11 | 2.10e+11 | 8.12e+10 | 5.61e+10 | 1.08e+11 | 7.60e+10 | 1.32e+11 | 2.14e+11 | 2.92e+10 |
| 1955 | 1.934e+12 | +0.5% | 3.03e+11 | 2.10e+11 | 8.15e+10 | 5.64e+10 | 1.08e+11 | 7.61e+10 | 1.32e+11 | 2.15e+11 | 2.94e+10 |

Obs.: o rótulo de ano da `partida_longa` repete "1948" (anos de 365 dias, pré-existente); os valores seguem o dia da simulação.

## 5. Custo por dia da simulação (a economia não ficou mais lenta)

| Medição | antes | depois |
|---|---|---|
| `partida_longa`, perfil dev, economia + IA, média de 10 anos (ms/dia) | 23,39 (medido em 20 anos) | **23,29** |
| `crescimento`, release, 1 thread, 2 anos (ms/dia: sim + IA) | 41,0 (36,1 + 4,9) | 41,4-42,2 (37,1-37,8 + 4,3-4,4) |
| `crescimento`, release, vários threads, 4 anos (ms/dia) | 27,1-27,6 (22,0 + 5,1) | 28,9-31,4 (24,2-26,4 + 4,7-5,0) |

A economia depois tem mais entradas de prédios e mais gente empregada, então a medição em release em vários anos fica
~5-10% acima (máquina compartilhada, ruído de +-15%); no perfil dev da partida completa ficou igual. As otimizações do item 7 compensam.
Fases (release, 4 anos, ms/dia): produção 9,1 -> ~9,7; distribuição 3,9 -> ~4,2; consumo 3,5 -> 3,4; trabalho 0,85 -> ~1,1; governo 3,2 -> ~3,7.

## 6. Limites conhecidos

- O primeiro ano ainda tem +9% (recuperação inicial). Se for alto demais, baixar `META_CONSTRUCAO` e os multiplicadores de `RECONSTRUCAO`.
- China cresce pouco (2,4%): guerra civil, alfabetização 0,2-0,4 e poucas vagas; é coerente até 1949, depois pode ganhar evento.
- Jogador: a IA não toca no país do jogador, então ele só recebe a produtividade (endógena + tecnologia) e a demografia; a capacidade de
  construção ele escolhe construir (o `setor_construcao` agora é estatal).
- Países com leis que proíbem investimento privado (planificação) não usam o fundo privado parado (ex.: GER/JAP/CHI/SOV ficam com ~3-6e9);
  só crescem pelas obras estatais.
