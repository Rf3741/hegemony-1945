# Relatório de Desempenho e Memória — Simulação Econômica (1945)

## 1. Contexto e Objetivos

O jogo *Guerra Fria* (1945) roda a simulação econômica completa (`sim_economia`, padrão Victoria 3) com o cenário real carregado de `assets/data/economia`. O cenário inicial conta com:
- **4.594 províncias**
- **941 estados**
- **155 países**
- **71 mercados regionais e 1 mercado global integrado**
- **~101.000 grupos populacionais (pops)** distribuídos nas províncias
- **~96.000 prédios produtivos e governamentais** ativos

A máquina de execução possui **7 GB de memória RAM total**.
As restrições de desempenho definidas para a simulação são:
- **Pico de memória (RSS / VmHWM):** < 1,5 GB
- **Tempo médio de tick diário:** < 30,0 ms/dia
- **Estabilidade numérica:** Ausência de NaNs, preços limitados e consistentes (faixa [0,25x; 1,75x] do preço base), PIBs de EUA e URSS positivos e calibrados historicamente.

---

## 2. Metodologia de Medição

Criou-se o binário de medição [medir_desempenho.rs](file:///home/rafael/claude/guerra-fria-jogar/crates/sim_economia/examples/medir_desempenho.rs), que executa o mesmo fluxo de carga e simulação do cliente de jogo:
1. Leitura direta dos arquivos `.ron` modulares (`CenarioEconomia::carregar_diretorio`).
2. Configuração de cadência otimizada (`ModoCadencia::DiarioComAjusteSemanal`, demografia e mercado de trabalho semanais com produção e preços diários).
3. Compilação e indexação em memória via `MundoEconomico::novo`.
4. Inicialização do `MercadoGlobal` com acompanhamento do mercado nacional dos EUA.
5. Aquecimento inicial de 90 dias (para estabilizar preços e oferta antes da interação).
6. Simulação contínua de 5 anos completos (1.826 dias, de 1945 a 1949).
7. Leitura em tempo real do `/proc/self/status` coletando `VmRSS` (Resident Set Size) e `VmHWM` (Peak Resident Set Size / High Water Mark).

Adicionalmente, avaliou-se o teste headless do cliente [economia_sim.rs](file:///home/rafael/claude/guerra-fria-jogar/crates/guerra_fria/src/plugins/economia_sim.rs) (`cinco_anos_headless_sem_nan_e_precos_na_faixa`), que inclui a geração a cada 30 dias do snapshot de interface `VistaEconomia` e auditorias de estabilidade.

---

## 3. Diagnóstico da Memória da Máquina

Durante execuções anteriores sem restrição de concorrência (`cargo test` padrão sem `--test-threads=1`), o Cargo executava múltiplos testes simultaneamente, compilando e linkando binários de teste pesados do Bevy em paralelo, além de preencher o *Page Cache* do kernel Linux com arquivos do `target`.

Inspeção do `/proc/meminfo`:
- **MemTotal:** 7.332.688 kB (~7,0 GB)
- **Cached (Page Cache do Linux):** ~4.903.072 kB (~4,9 GB)
- **MemAvailable (Memória realmente disponível para alocação):** ~5.365.204 kB (~5,1 GB)

Comandos com `CARGO_BUILD_JOBS=2` e execução de testes com `--test-threads=1` eliminam a disputa de memória e garantem consumo previsível.

---

## 4. Resultados Medidos

### 4.1. Medição Passo a Passo do Motor (`sim_economia`)

| Etapa | VmRSS (Atual) | VmHWM (Pico) | Tempo Decorrido | Média por Dia |
|---|---|---|---|---|
| Início do processo | 2 MB | 2 MB | 0 ms | — |
| Carga e parse dos arquivos `.ron` | 86 MB | 110 MB | 671 ms | — |
| `MundoEconomico::novo` (indexação e compilação de prédios) | 88 MB | 110 MB | 65 ms | — |
| Aquecimento de 90 dias | 90 MB | 110 MB | 1,80 s | 20,02 ms/dia |
| Ano 1 (dia 365 / 1.826) | 92 MB | 110 MB | 7,44 s | 20,38 ms/dia |
| Ano 2 (dia 730 / 1.826) | 92 MB | 110 MB | 14,86 s | 20,36 ms/dia |
| Ano 3 (dia 1.095 / 1.826) | 93 MB | 110 MB | 22,32 s | 20,38 ms/dia |
| Ano 4 (dia 1.460 / 1.826) | 94 MB | 110 MB | 29,83 s | 20,43 ms/dia |
| Ano 5 (dia 1.826 / 1.826) — Final | 94 MB | 110 MB | 37,30 s | 20,43 ms/dia |

- **Pico Máximo de Memória (VmHWM):** **110 MB** (Limite: 1.500 MB — margem de folga de 13,6x).
- **Tempo Médio do Tick:** **20,43 ms/dia** (Limite: 30,0 ms/dia — margem de folga de 32%).
- **Pior Tick Isolado (Tick Máximo):** 42,9 ms (ocorre no tick de ajuste semanal demográfico).

### 4.2. Medição do Cliente Bevy Headless (`guerra_fria`)

- **Teste:** `cinco_anos_headless_sem_nan_e_precos_na_faixa`
- **Operações:** 90 dias aquecimento + 1.826 dias simulados em lotes de 30 dias + comandos de construção, política fiscal e tarifas + reconstrução de `VistaEconomia` a cada lote + auditoria de estabilidade numérica.
- **Tempo total:** 40,88 s (~21,9 ms/dia incluindo overhead do ECS e snapshot da interface).
- **Resultado:** Aprovado sem falhas, sem vazamento de memória e sem degradação ao longo dos 5 anos.

---

## 5. Calibração Econômica Observada (Fim dos 5 Anos)

| Indicador | Estados Unidos (EUA) | União Soviética (URSS) |
|---|---|---|
| **PIB Diário** | $ 1.240.340.272,50 | $ 155.685.851,33 |
| **PIB Anualizado** | ~ $ 452,7 bilhões | ~ $ 56,8 bilhões |
| **Tesouro Acumulado** | $ 116,4 bilhões | $ 21,0 bilhões |
| **Padrão de Vida Médio (SoL)** | 9,2 | 5,3 |
| **Preços Mundiais e Nacionais** | Estáveis, sem NaNs | Estáveis, sem NaNs |

---

## 6. Conclusão sobre Agrupamento de Pops e Otimizações

Como o pico de memória medido no cenário real completo foi de **110 MB** (bem abaixo do teto de 1.500 MB) e o tempo médio do tick foi de **20,4 ms** (abaixo do teto de 30 ms):
1. **Não foi necessário degradar os pops por estado:** Manter os pops no nível de província preserva a fidelidade cartográfica, a atração migratória local e a calibração de emprego de cada prédio da província.
2. **As estruturas pré-compiladas existentes (`PredioCompilado` inline com arrays de tamanho fixo `[MAX_FLUXOS_PREDIO]` e `[MAX_VAGAS_PREDIO]`) já operam com zero alocações dinâmicas no loop quente do tick.**
3. **A cadência `ModoCadencia::DiarioComAjusteSemanal` ativada em `Nucleo::carregar` garante ganho de ~25% sem distorção nos fluxos diários de mercadorias e formação de preços.**
4. A execução dos testes com `--test-threads=1` e compilação com `CARGO_BUILD_JOBS=2` é obrigatória para evitar esgotamento de memória por concorrência de compilação.
