# DESIGN.md — Guerra Fria: Grande Estratégia Geopolítica (1945–1991)

## 1. Visão Geral

**Guerra Fria** é um jogo de grande estratégia geopolítica, macroeconômica e sociopolítica em tempo real pausável para desktop (**Linux** e **Windows**), construído em **Rust + Bevy + `bevy_egui`**, inspirado na profundidade sistêmica de *Victoria 3* e na escala cartográfica acessível de *Age of History* / *Hearts of Iron*.

- **Data Inicial**: **1º de janeiro de 1945** — os meses finais da Segunda Guerra Mundial e o alvorecer da ordem bipolar. Os Estados Unidos emergem com capacidade industrial intacta e hegemonia financeira, enquanto a União Soviética avança sobre o Leste Europeu com o maior exército terrestre do planeta, mas com sua porção ocidental devastada. Impérios coloniais europeus (Britânico, Francês, Holandês, Belga, Português) enfrentam exaustão fiscal e movimentos de libertação nacional na Ásia e na África.
- **Escala Temporal**: Tempo real com pausa (**`Espaço`** para pausar/retomar, velocidades **`1` a `5`**), com **tick diário** determinístico e consolidações semanais/mensais de balanço econômico e político.
- **Escala Geográfica**: Mundo real completo com **4.596 províncias terrestres** (baseadas no *Natural Earth 10m Admin-1*), agrupando metrópoles, bacias industriais, regiões agrárias, colônias e zonas de ocupação militar.
- **Idioma**: Todo o jogo, interface, dados e documentação em **português brasileiro (`pt-BR`)**.
- **Fantasia Central do Jogador**: Conduzir qualquer nação ou território de 1945 — de superpotências (EUA, URSS) a potências em reconstrução (Reino Unido, França, Alemanha, Japão, China) ou nações emergentes e anticoloniais (Brasil, Índia, Indonésia, Egito) — equilibrando industrialização, padrão de vida da população, grupos de interesse internos, reformas constitucionais e alinhamento diplomático entre o capitalismo ocidental e o socialismo soviético.

---

## 2. Pilares de Design

1. **Materialismo Sistêmico (Nada Surge do Vazio)**
   Cada fuzil, locomotiva, pão ou rádio consumido no mundo é produzido por um **prédio** específico, utilizando **insumos** comprados num **mercado**, operado por **Pops** (grupos populacionais) que recebem salários, pagam impostos e compram bens para sustentar seu **Padrão de Vida (*SoL*)**.
2. **Política Emergente da Estrutura Social**
   Grupos de Interesse (*IGs*) não têm poder fixo por script: sua força política (*Clout*) nasce da riqueza, alfabetização e demografia das profissões que os compõem. Industrializar um país agrário enfraquece inevitavelmente os Proprietários Rurais e fortalece Industriais e Trabalhadores Sindicalizados.
3. **Bipolaridade Assimétrica (Capitalismo vs. Comunismo)**
   Economias capitalistas operam via sinais de preço, lucro privado, dividendos e *Fundo de Investimento* autônomo; economias planificadas operam via metas estatais (*Gosplan*), requisição/apropriação de excedentes, pleno emprego administrativo e alocação direta de construção estatal, enfrentando gargalos burocráticos distintos.
4. **Descolonização e Soberania no Pós-1945**
   Mais de metade da população mundial em 1945 vive sob domínio colonial ou ocupação militar. O custo administrativo e a radicalização das populações coloniais transformam o mapa político ao longo das décadas.
5. **Simulação Pura e Determinística**
   Toda a regra de negócio reside na crate pura `simulacao` (sem dependência de motor gráfico), permitindo testes unitários rápidos, reprodutibilidade exata e clara separação entre estado lógico e renderização Bevy.

---

## 3. Roadmap de Desenvolvimento

### Fase / Versão 1 (`v1`) — Política Interna, Economia e Mapa Político Global *(Foco Atual)*
- Pipeline cartográfico completo (*Natural Earth 10m Admin-1* $\to$ `provinces.png` + `provinces.ron` + cenário histórico `scenario_1945`).
- Renderização 2D acelerada por GPU (textura de IDs + textura de *lookup* por modo de mapa, fronteiras internacionais e provinciais, câmera com pan/zoom, inspeção por clique).
- Relógio em tempo real pausável (velocidades 1–5, tick diário a partir de `01/01/1945`).
- Motor econômico completo: Pops estratificadas, 25 bens, 18 tipos de prédios, mercados nacionais e blocos comerciais, precificação por oferta/demanda, salários, consumo, orçamento fiscal, impostos e fila de construção.
- Motor político completo: 7 categorias institucionais (40 leis), 11 Grupos de Interesse, cálculo de legitimidade, eleições periódicas, radicalismo/lealdade e esferas de influência EUA/URSS.

### Versão 2 (`v2`) — Guerra Convencional, Guerras por Procuração e Dissuasão Nuclear
- Mobilização de recrutas e exércitos profissionais abastecidos por cadeias logísticas reais (armas leves, munição, combustível, blindados, aeronaves).
- Sistema de **Frentes e Teatros** (estilo *Victoria 3* adaptado à guerra mecanizada pós-1945), bloqueios navais e atrito logístico.
- **Guerras por Procuração (*Proxy Wars*)**, insurgências anticoloniais, envio de conselheiros militares, financiamento clandestino e golpes de Estado apoiados por agências de inteligência (CIA / KGB).
- **Dissuasão Nuclear**: arsenal estratégico (bombardeiros, ICBMs, SLBMs), doutrina MAD (*Destruição Mútua Assegurada*) e escala de crise global (**DEFCON 5 $\to$ 1**).

### Versão 3 (`v3`) — Tecnologia, Corrida Espacial e Era da Informação
- **Árvore Tecnológica** dividida em 3 ramos com difusão internacional de conhecimento (*Tech Spread*) e pesquisa ativa em Universidades:
  1. *Produção e Ciência*: Petroquímica avançada, Energia Nuclear Civil, Revolução Verde, Automação Industrial, Semicondutores e Computação.
  2. *Doutrina Militar e Aeroespacial*: Propulsão a Jato, Mísseis Balísticos, Satélites Espiões, Guerra Assimetrica, Armas Guiadas.
  3. *Sociedade, Mídia e Estado*: Televisão em Massa, Estado de Bem-Estar Social, Cibernética Econômica, Direitos Civis Contemporâneos, Globalização Financeira.
- **Corrida Espacial** e Projetos de Prestígio Global (primeiro satélite, voo tripulado, pouso lunar, estações orbitais).

---

## 4. Design da Economia (`v1`)

### 4.1. População (*Pops*)
Uma **Pop** representa uma coorte homogênea de habitantes definida pela tupla `(provincia_id, profissao, cultura, religiao)`.

**Atributos de cada Pop:**
- `populacao_ativa`: trabalhadores adultos aptos a ocupar vagas nos prédios ($N_{\text{ativos}}$).
- `dependentes`: crianças, idosos e cônjuges fora da força de trabalho formal ($N_{\text{dep}} \approx 1.5 \cdot N_{\text{ativos}}$, ajustado por leis trabalhistas e saúde).
- `alfabetizacao` $\in [0.0, 1.0]$: determina a taxa máxima de qualificação para profissões técnicas, burocráticas e acadêmicas.
- `padrao_vida` (*Standard of Living — SoL*) $\in [1.0, 99.0]$: índice material calculado pela satisfação da cesta de consumo.
  - Faixas de referência: $1\text{–}9$ (*Miséria*), $10\text{–}14$ (*Luta pela Sobrevivência*), $15\text{–}24$ (*Padrão Médio Urbano*), $25\text{–}39$ (*Próspero*), $40+$ (*Opulento*).
- `riqueza_acumulada`: poupança líquida da Pop em moeda corrente ($\$$).
- `radicais` e `leais`: frações da Pop politicamente mobilizadas contra ou a favor do regime vigente.

**As 12 Profissões e seus Estratos:**

| Estrato | ID (`snake_case`) | Nome em Jogo | Onde Trabalha | Multiplicador Salarial Base ($\omega_p$) |
| :--- | :--- | :--- | :--- | :---: |
| **Baixo** | `camponeses` | Camponeses | Fazendas de Subsistência | $0.4\times$ *(renda in natura)* |
| **Baixo** | `trabalhadores_rurais` | Trabalhadores Rurais | Fazendas, Minas, Serrarias, Seringais | $1.0\times$ |
| **Baixo** | `operarios` | Operários | Indústrias, Construção, Portos | $1.3\times$ |
| **Baixo** | `soldados` | Soldados | Quartéis, Bases Navais | $1.2\times$ |
| **Médio** | `maquinistas` | Técnicos e Maquinistas | Indústria Pesada, Ferrovias, Usinas | $2.0\times$ |
| **Médio** | `escriturarios` | Escriturários | Centros Urbanos, Comércio, Administração | $1.8\times$ |
| **Médio** | `burocratas` | Burocratas | Administração Governamental | $2.4\times$ |
| **Médio** | `academicos` | Acadêmicos e Engenheiros | Universidades, Complexos Científicos | $3.0\times$ |
| **Médio** | `clerigos` | Clérigos | Instituições Religiosas, Escolas Confessionais | $1.8\times$ |
| **Médio** | `oficiais` | Oficiais Militares | Quartéis, Estados-Maiores | $3.2\times$ |
| **Alto** | `aristocratas` | Latifundiários | Fazendas e Plantações (Propriedade Rural) | $5.0\times$ + Dividendos |
| **Alto** | `capitalistas` | Capitalistas / Diretores | Indústrias, Minas, Finanças (*ou Quadros Gosplan*) | $6.0\times$ + Dividendos |

**Crescimento Demográfico Anual ($\Delta N / N$):**
$$\text{TaxaAnual}(\text{SoL}) = \text{Natalidade}(\text{SoL}, \text{Alfabetização}) - \text{Mortalidade}(\text{SoL}, \text{Saúde})$$
- Abaixo de $\text{SoL} = 8$: mortalidade supera natalidade (fome/crise humanitária, até $-2.0\%$/ano).
- Entre $\text{SoL} = 10$ e $22$: pico da transição demográfica (*Baby Boom* pós-1945, $+1.4\%$ a $+2.6\%$/ano).
- Acima de $\text{SoL} = 28$ com alta alfabetização: desaceleração gradual da natalidade ($+0.5\%$ a $+1.1\%$/ano).

---

### 4.2. Lista Completa de Bens (25 Bens)

Todos os bens possuem um **Preço Base ($P_0$)** de referência. Bens **Locais** (Eletricidade, Transporte, Serviços Urbanos) são precificados na própria província/estado; os demais são negociados no **Mercado Nacional / Bloco Comercial**.

| # | ID | Nome (pt-BR) | Categoria | $P_0$ | Uso Principal |
| :-: | :--- | :--- | :--- | :-: | :--- |
| 1 | `graos` | Grãos e Cereais | Básico / Agrícola | 20 | Alimentação básica, insumo de alimentos processados |
| 2 | `carne_pescado` | Carne e Pescado | Básico / Agrícola | 30 | Alimentação proteica, conservas |
| 3 | `tecidos` | Algodão e Fibras | Básico / Agrícola | 20 | Insumo têxtil, vestuário simples |
| 4 | `madeira` | Madeira | Extrativo | 20 | Construção básica, móveis, papel, aquecimento |
| 5 | `borracha` | Borracha | Extrativo | 40 | Automóveis, aviação, equipamentos elétricos |
| 6 | `carvao` | Carvão Mineral | Energia / Extrativo | 30 | Siderurgia, termelétricas, aquecimento |
| 7 | `ferro` | Minério de Ferro | Extrativo | 40 | Insumo principal de siderúrgicas |
| 8 | `cobre` | Cobre | Extrativo | 45 | Eletrificação, rádios/eletrodomésticos, munições |
| 9 | `bauxita` | Bauxita | Extrativo | 45 | Produção de alumínio e indústria aeronáutica |
| 10 | `petroleo` | Petróleo Cru | Energia / Extrativo | 50 | Refino de combustível, petroquímica |
| 11 | `uranio` | Minério de Urânio | Estratégico | 100 | Programas estratégicos e energia nuclear futura |
| 12 | `aco` | Aço | Industrial | 50 | Construção moderna, máquinas, ferrovias, blindados, navios |
| 13 | `aluminio` | Alumínio e Ligas | Industrial | 65 | Aeronáutica, eletrodomésticos modernos |
| 14 | `quimicos` | Produtos Químicos | Industrial | 50 | Fertilizantes agrícolas, explosivos/munições, sintéticos |
| 15 | `combustivel` | Combustível Refinado | Industrial / Energia | 60 | Transporte motorizado, mecanização, forças armadas |
| 16 | `ferramentas` | Ferramentas e Máquinas | Industrial | 55 | Manutenção de todas as minas, fábricas e setor de construção |
| 17 | `cimento` | Cimento e Materiais | Industrial | 35 | Setor de construção civil e infraestrutura |
| 18 | `eletricidade` | Eletricidade *(Local)* | Infraestrutura | 30 | Energia limpa para indústrias e cidades |
| 19 | `transporte` | Transporte *(Local)* | Infraestrutura | 30 | Escoamento ferroviário/portuário e mobilidade urbana |
| 20 | `alimentos_proc` | Alimentos Processados | Consumo Civil | 35 | Alimentação urbana de estrato médio/alto e rações |
| 21 | `roupas` | Roupas e Calçados | Consumo Civil | 35 | Vestuário padrão da população |
| 22 | `moveis` | Móveis e Utensílios | Consumo Civil | 40 | Habitação doméstica |
| 23 | `eletrodomesticos` | Rádios e Eletrodomésticos | Consumo / Luxo | 70 | Consumo de classe média/alta, difusão cultural |
| 24 | `automoveis` | Automóveis e Caminhões | Consumo / Luxo | 100 | Consumo de alto padrão e logística rodoviária |
| 25 | `servicos` | Serviços Urbanos *(Local)* | Consumo Civil | 30 | Comércio, lazer, saúde/educação privada nas cidades |
| 26 | `armas_municoes` | Armamentos e Munições | Militar | 65 | Manutenção de quartéis, guarnições e prontidão militar |

---

### 4.3. Prédios e Cadeias Produtivas

Cada nível de prédio emprega **5.000 trabalhadores** em plena ocupação (exceto Centros Urbanos e Subsistência, escalados pela demografia).

| ID do Prédio | Nome (pt-BR) | Custo Construção | Insumos por Nível (Base) | Saídas por Nível (Base) |
| :--- | :--- | :---: | :--- | :--- |
| `fazenda_subsistencia` | Agricultura de Subsistência | — *(Automático)* | — | `graos` (12), `tecidos` (4), `servicos` (2) |
| `fazenda` | Fazenda Comercial | 200 | `ferramentas` (5), `quimicos` (5) | `graos` (40), `carne_pescado` (15) |
| `plantacao` | Plantação (Fibras/Borracha) | 200 | `ferramentas` (5) | `tecidos` (30), `borracha` (15) |
| `serraria` | Serraria | 180 | `ferramentas` (5) | `madeira` (45) |
| `porto_pesqueiro` | Porto Pesqueiro | 180 | `madeira` (5), `ferramentas` (3) | `carne_pescado` (35) |
| `mina_carvao` | Mina de Carvão | 300 | `ferramentas` (10) | `carvao` (50) |
| `mina_ferro` | Mina de Ferro | 320 | `ferramentas` (10), `carvao` (5) | `ferro` (42) |
| `mina_metais` | Mina de Cobre e Bauxita | 350 | `ferramentas` (12), `carvao` (5) | `cobre` (24), `bauxita` (20) |
| `mina_uranio` | Mina de Urânio | 500 | `ferramentas` (15), `quimicos` (10), `eletricidade` (10) | `uranio` (15) |
| `campo_petrolifero` | Campo Petrolífero | 450 | `ferramentas` (15), `aco` (5) | `petroleo` (50) |
| `siderurgica` | Siderúrgica e Metalurgia | 550 | `ferro` (30), `carvao` (25), `bauxita` (10), `eletricidade` (10) | `aco` (50), `aluminio` (18) |
| `refinaria_quimica` | Refinaria e Indústria Química | 550 | `petroleo` (35), `carvao` (15), `eletricidade` (10) | `combustivel` (42), `quimicos` (35) |
| `fabrica_maquinas` | Fábrica de Ferramentas e Cimento | 500 | `aco` (25), `madeira` (15), `carvao` (10) | `ferramentas` (45), `cimento` (35) |
| `industria_alimentos` | Indústria Alimentícia | 400 | `graos` (25), `carne_pescado` (15), `ferramentas` (5) | `alimentos_proc` (55) |
| `industria_consumo` | Indústria Têxtil e Moveleira | 420 | `tecidos` (25), `madeira` (20), `ferramentas` (5) | `roupas` (40), `moveis` (32) |
| `industria_eletro_auto` | Indústria Automotiva e Eletrônica | 650 | `aco` (20), `aluminio` (10), `borracha` (12), `cobre` (12), `combustivel` (8) | `eletrodomesticos` (28), `automoveis` (18) |
| `industria_belica` | Indústria de Armamentos | 600 | `aco` (22), `quimicos` (18), `cobre` (10), `ferramentas` (8) | `armas_municoes` (45) |
| `usina_eletrica` | Usina Termo/Hidrelétrica | 450 | `carvao` (20), `ferramentas` (8) | `eletricidade` (75) |
| `ferrovia_porto` | Ferrovia e Porto Comercial | 450 | `aco` (10), `combustivel` (12), `ferramentas` (6) | `transporte` (60) + **+25 Infraestrutura** |
| `centro_urbano` | Centro Urbano | — *(Automático)* | `eletricidade` (8), `carvao` (6) | `servicos` (50) |
| `administracao` | Administração Governamental | 350 | `madeira` (8), `servicos` (5) | **+50 Capacidade Burocrática** |
| `quartel` | Quartel e Base Militar | 300 | `armas_municoes` (12), `combustivel` (6), `alimentos_proc` (8) | **+10 Projeção de Poder / Ordem** |
| `setor_construcao` | Setor de Construção | 250 | `cimento` (20), `aco` (15), `madeira` (15), `ferramentas` (10) | **+20 Pontos de Construção / semana** |

**Fórmula de Volume Efetivo de Produção ($Q_{\text{saida}}$):**
$$Q_{\text{saida}} = N_{\text{nivel}} \cdot q_{\text{base}} \cdot \left(\frac{L_{\text{empregado}}}{L_{\text{max}}}\right) \cdot E_{\text{escala}} \cdot (1 - D_{\text{guerra}}) \cdot (1 - \phi_{\text{escassez}})$$
onde:
- **Economia de Escala**: $E_{\text{escala}} = 1 + 0.01 \cdot \min(N_{\text{nivel}} - 1,\ 50)$ (até $+50\%$ de throughput em grandes complexos industriais como Detroit, Ruhr ou Magnitogorsk).
- **Destruição de Guerra**: $D_{\text{guerra}} \in [0, 0.85]$ decai gradualmente em tempo de paz ($-0.0008$/dia, ou mais rápido com construção ativa).
- **Penalidade de Escassez Física ($\phi_{\text{escassez}}$)**: se no mercado a oferta de um insumo essencial for menor que metade da demanda ($O_i < 0.5 D_i$), $\phi_{\text{escassez}}$ cresce até $0.75$.

---

### 4.4. Mercados, Acesso de Infraestrutura e Formação de Preços

Cada país soberano lidera seu **Mercado Nacional**, ou participa do **Mercado de seu Suserano / Bloco Econômico** (ex.: Índia Britânica, Nigéria e Austrália integram a *Área da Libra Esterlina* liderada por `GBR`; países do Leste Europeu podem integrar o bloco soviético liderado por `SOV`).

**1. Acesso ao Mercado da Província ($M_r$):**
$$M_r = \text{clamp}\left(\frac{\text{Infraestrutura}_r}{\max(1,\ \text{InfraUsada}_r)},\ 0.15,\ 1.00\right)$$
Se uma província tem infraestrutura insuficiente (ou destruída por bombardeios em 1945), suas ordens de compra e venda chegam apenas parcialmente ($M_r$) ao mercado nacional, gerando divergência de preço local:
$$P_{\text{local}}(r, b) = M_r \cdot P_{\text{mercado}}(b) + (1 - M_r) \cdot P_{\text{autarquico}}(r, b)$$

**2. Fórmula de Preço de Mercado (Estilo *Victoria 3*):**
Seja $O_b = \sum_r M_r \cdot \text{Vendas}(r, b) + \text{Importações}(b)$ a oferta efetiva, e $D_b = \sum_r M_r \cdot \text{Compras}(r, b) + \text{Exportações}(b)$ a demanda efetiva do bem $b$. O preço-alvo $P^*(b)$ é:
$$P^*(b) = P_0(b) \cdot \text{clamp}\left(1 + 0.75 \cdot \frac{D_b - O_b}{\max(\varepsilon,\ \min(D_b, O_b))},\ 0.25,\ 1.75\right)$$
- Preço mínimo: **$0.25 \cdot P_0$** ($-75\%$, superabundância).
- Preço de equilíbrio ($D_b = O_b$): **$1.00 \cdot P_0$**.
- Preço máximo: **$1.75 \cdot P_0$** ($+75\%$, escassez severa).
- Para evitar oscilações bruscas de um dia para o outro, o preço corrente $P_t(b)$ converge suavemente a cada tick diário:
  $$P_{t+1}(b) = P_t(b) + \alpha \cdot \left(P^*(b) - P_t(b)\right), \quad \alpha = 0.20$$

**3. Lucro, Reserva de Caixa e Fundo de Investimento:**
$$\Pi_{\text{predio}} = \sum_{s \in \text{Saídas}} Q_s P_s - \sum_{i \in \text{Insumos}} Q_i P_i - \text{FolhaSalarial}$$
- Se $\Pi > 0$: parte abastece a **Reserva de Caixa** do prédio (até o teto de segurança) e o excedente é distribuído como **Dividendos** aos proprietários (Capitalistas/Aristocratas no capitalismo; Tesouro/Fundo *Gosplan* na economia planificada).
- Uma fração dos dividendos capitalistas ($25\%$ no Intervencionismo, $40\%$ no Livre Mercado) alimenta automaticamente o **Fundo Privado de Investimento**, que financia construções privadas lucrativas.

---

### 4.5. Orçamento Nacional, Impostos e Dívida Soberana

**Eficiência de Arrecadação ($\eta_{\text{fisco}}$):**
$$\eta_{\text{fisco}} = \text{clamp}\left(\frac{\text{CapacidadeBurocratica}}{\max(1,\ \text{CustoBurocraticoPopLeis})},\ 0.25,\ 1.00\right)$$
Déficit burocrático reduz diretamente a coleta de impostos ("evasão e desorganização administrativa").

**Fontes de Receita Semanal/Diária ($R_{\text{total}}$):**
1. **Imposto de Renda ($T_{\text{renda}}$)**: alíquota $\tau_{\text{renda}} \in [0\%, 35\%]$ sobre salários formais.
2. **Imposto sobre Consumo ($T_{\text{consumo}}$)**: alíquota $\tau_{\text{cons}}$ sobre gastos das Pops em bens básicos ou de luxo.
3. **Imposto sobre Dividendos e Lucros ($T_{\text{div}}$)**: alíquota progressiva $\tau_{\text{div}} \in [0\%, 50\%]$ sobre lucros distribuídos (ou **100% de Apropriação do Excedente Operacional** em `economia_planificada`).
4. **Imposto Territorial / Per Capita ($T_{\text{cap}}$)**: cobrado sobre camponeses e propriedades rurais em sistemas tributários regressivos/coloniais.
5. **Tarifas Alfandegárias e Transferências de Vassalos/Colônias ($T_{\text{ext}}$)**: $15\%$ da receita bruta de colônias (`governador_colonial`) é remetida à metrópole.

**Despesas Governamentais ($G_{\text{total}}$):**
1. **Salários e Bens Administrativos**: folha dos `burocratas` + insumos das Administrações e Universidades.
2. **Orçamento Militar**: folha de `soldados` e `oficiais` + compra de `armas_municoes` e `combustivel`.
3. **Construção Estatal**: custo dos insumos e mão de obra dos setores de construção alocados na fila pública.
4. **Seguridade Social e Subsídios**: compensação a prédios estratégicos deficitários e pensões em leis trabalhistas avançadas.
5. **Serviço da Dívida ($J_{\text{divida}}$)**:
   $$J_{\text{diario}} = \text{Divida} \cdot \frac{r_{\text{anual}}}{365}, \quad r_{\text{anual}} = r_{\text{base}} + 0.06 \cdot \left(\frac{\text{Divida}}{\text{LimiteCredito}}\right)^2$$
   onde $r_{\text{base}} = 4\%$ para grandes potências ($8\%$ para países periféricos) e $\text{LimiteCredito} = 0.50 \cdot \text{PIB}_{\text{anual}} + \sum \text{ReservasPredios}$. Ultrapassar $100\%$ do limite desencadeia **Moratória (*Default*)**, colapsando o throughput produtivo ($-40\%$) e o prestígio diplomático.

---

## 5. Design da Política (`v1`)

### 5.1. As 7 Categorias Institucionais de Leis (40 Leis)

Cada país possui exatamente **1 lei ativa** em cada uma das 7 categorias abaixo:

#### 1. Distribuição de Poder (`governo`)
- `autocracia`: Poder concentrado no chefe de Estado; $+30\%$ força política de Aristocratas/Militares, sem eleições.
- `oligarquia`: Governança de elites econômicas e militares; $+25\%$ força política de Industriais, Proprietários Rurais e Militares.
- `voto_censitario`: Voto ponderado pela riqueza das Pops; favorece estrato alto e médio.
- `voto_capacitario`: Voto restrito a Pops alfabetizadas; fortalece Intelectuais e Pequena Burguesia.
- `sufragio_universal`: Um cidadão, um voto; eleições a cada 4 anos, alto peso demográfico de Sindicatos e População Rural.
- `partido_unico`: Monopólio político do partido de vanguarda; $+50\%$ força da Nomenklatura, suprime oposição liberal/clerical.
- `governador_colonial`: Administração nomeada pela metrópole; $+40\%$ Administração Colonial, penaliza legitimidade entre culturas nativas não aceitas.
- `ocupacao_militar`: Governo militar aliado pós-guerra (ex.: Alemanha e Japão em 1945); autoridade coercitiva elevada, soberania externa suspensa.

#### 2. Sistema Econômico (`economia`)
- `tradicionalismo_agrario`: $-25\%$ eficiência de coleta fiscal, sem fundo de investimento moderno.
- `livre_mercado`: $+40\%$ contribuição capitalista ao Fundo de Investimento, proíbe subsídios a indústrias civis, tarifas reduzidas.
- `capitalismo_de_intervencao`: Equilíbrio misto; permite subsídios, construção estatal + privada ($25\%$ reinvestimento capitalista).
- `economia_de_guerra`: $+25\%$ throughput de `industria_belica`, $+15\%$ impostos extraordinários, $+20\%$ custo de bens civis (ativa em janeiro de 1945 nas grandes potências beligerantes).
- `economia_planificada`: Propriedade estatal dos prédios urbanos, $100\%$ do investimento via orçamento público (*Gosplan*), subsídio automático de pleno emprego, $+20\%$ custo burocrático.
- `extrativismo_colonial`: $+20\%$ produção de minas e plantações, $-30\%$ qualificação industrial local, transfere dividendos à metrópole.

#### 3. Relações Trabalhistas (`trabalho`)
- `trabalho_forcado_ou_escravo`: Salários mínimos nulos em extração/campos, altíssimo radicalismo e mortalidade.
- `trabalho_livre_desregulamentado`: Sem piso salarial; $+10\%$ margem de lucro empresarial, maior mortalidade operária.
- `regulamentacao_basica`: $-20\%$ mortalidade em minas/fábricas, piso salarial de $60\%$ da média.
- `direitos_trabalhistas_avancados`: Piso salarial de $90\%$, $+15\%$ força política dos Trabalhadores Sindicalizados, seguridade social.
- `trabalhismo_estatal`: Sindicatos corporativos atrelados ao Estado (ex.: CLT varguista); reduz greves ($-25\%$ radicalismo operário), $+10\%$ poder dos Burocratas.
- `controle_sindical_obrero`: Sindicatos integrados às metas produtivas do partido único; $+10\%$ throughput fabril estatal, greves proibidas.

#### 4. Direitos Civis e Segurança (`direitos_civis`)
- `estado_policial`: $-50\%$ impacto político de movimentos radicais, $+30\%$ supressão policial, $-15\%$ atração migratória e difusão acadêmica.
- `censura_estrita`: $-25\%$ crescimento de movimentos oposicionistas, permite banir grupos extremistas.
- `liberdades_restringidas`: Ordem pública moderada ($-10\%$ radicalismo por revolta).
- `liberdade_de_expressao`: $+25\%$ geração de inovação/difusão cultural, impossibilita suprimir grupos de interesse à força.
- `lei_marcial_de_guerra`: Estado de sítio temporário; reduz distúrbios imediatos ao custo de desgaste contínuo de legitimidade em tempo de paz.

#### 5. Sistema Tributário (`sistema_tributario`)
- `tributacao_regressiva`: Alta taxação per capita e sobre consumo básico ($35\%$), baixa taxação de renda/dividendos ($5\%$).
- `tributacao_proporcional`: Alíquota equilibrada sobre renda ($20\%$) e dividendos ($15\%$).
- `tributacao_progressiva`: Foco em impostos de renda ($25\%$) e dividendos da elite ($35\%$), aliviando o consumo popular.
- `tributo_colonial`: Imposto per capita coercitivo sobre populações coloniais e tarifas de exportação para a metrópole.
- `requisicao_estatal`: Coleta direta do excedente das empresas estatais e cooperativas agrícolas coletivas.

#### 6. Saúde e Educação (`saude_e_educacao`)
- `inexistente_ou_privada`: Apenas Pops com $\text{SoL} \ge 15$ aumentam alfabetização por conta própria.
- `escolas_religiosas`: $+15\%$ alfabetização base, $+25\%$ força política do Clero, $-1.0\%$ mortalidade.
- `sistema_publico_basico`: $+30\%$ taxa de alfabetização, $-2.0\%$ mortalidade, $+10\%$ força dos Intelectuais.
- `sistema_publico_universal`: $+50\%$ taxa de alfabetização, $-4.0\%$ mortalidade (forte crescimento demográfico e qualificação técnica), alto custo burocrático.

#### 7. Organização Militar (`exercito`)
- `mobilizacao_total_de_guerra`: Conscrição máxima de guerra; alto consumo de `armas_municoes` e drenagem da mão de obra civil.
- `alistamento_obrigatorio`: Exército permanente médio com ampla reserva de conscritos.
- `exercito_profissional`: Menor efetivo bruto, $+20\%$ qualidade/moral, $+15\%$ força política dos Militares.
- `milicia_popular`: Baixo custo permanente em tempo de paz, alta defesa territorial interna.
- `forca_colonial`: Guarnições coloniais voltadas à manutenção da ordem imperial.
- `desmilitarizado`: Gastos militares mínimos ($\le 1\%$ do orçamento), imposto por tratados de rendição (Alemanha/Japão ocupados).

**Reforma de Leis:**
Propor uma nova lei inicia um ciclo de **3 marcos** (*Debate $\to$ Comissão/Votação $\to$ Promulgação*). A cada 30 ticks diários ocorre um avanço probabilístico:
- $P(\text{Avanço}) = \text{clamp}\left(\text{ApoioGoverno} + \text{ApoioMovimentos} - 0.5 \cdot \text{OposicaoAtiva} + \frac{\text{Legitimidade} - 50}{200},\ 0.05,\ 0.85\right)$.

---

### 5.2. Os 11 Grupos de Interesse (*IGs*)

Cada Pop ativa distribui seu **Poder Político Individual** ($\pi_{\text{pop}} = \text{Votos} + 0.25 \cdot \text{Riqueza} \cdot \text{Alfabetização}$) entre os Grupos de Interesse compatíveis com sua profissão:

| ID | Nome (pt-BR) | Base Social Principal | Pautas Típicas |
| :--- | :--- | :--- | :--- |
| `industriais` | Industriais | Capitalistas, Engenheiros | Livre Mercado / Intervencionismo, Tributação Proporcional, Alinhamento Ocidental |
| `trabalhadores_sindicalizados` | Trabalhadores Sindicalizados | Operários, Maquinistas | Direitos Trabalhistas, Saúde/Educação Universal, Tributação Progressiva ou Socialismo |
| `proprietarios_rurais` | Proprietários Rurais | Aristocratas/Latifundiários, Clérigos rurais | Oligarquia, Tributação Regressiva, Tradicionalismo Agrário |
| `populacao_rural` | População Rural | Camponeses, Trabalhadores Rurais | Reforma Agrária, Protecionismo Agrícola, Isolacionismo |
| `intelectuais` | Intelectuais | Acadêmicos, Escriturários, Burocratas | Liberdade de Expressão, Sufrágio Universal, Educação Pública |
| `militares` | Forças Armadas | Oficiais, Soldados | Exército Profissional/Alistamento, Ordem Pública, Indústria Bélica |
| `burocratas` | Burocratas de Estado | Burocratas, Administradores | Intervencionismo/Planificação, Capacidade Fiscal, Centralização |
| `clero` | Clero e Instituições Religiosas | Clérigos, Camponeses devotos | Escolas Religiosas, Censura Moral, Anticomunismo |
| `pequena_burguesia` | Pequena Burguesia | Escriturários, Comerciantes urbanos | Voto Censitário/Universal, Segurança Interna, Baixos Impostos |
| `nomenklatura_partidaria` | Nomenklatura Partidária | Quadros Estatais, Burocratas/Comissários | Partido Único, Economia Planificada, Estado Policial, Alinhamento Soviético |
| `administracao_colonial` | Administração Colonial | Oficiais e Burocratas metropolitanos | Governador Colonial, Extrativismo Colonial, Tributo Colonial |

**Força Política Relativa (*Clout*) do Grupo $g$:**
$$\text{Clout}(g) = \frac{\Pi_g \cdot (1 + \beta_{\text{leis}}(g))}{\sum_{k} \Pi_k \cdot (1 + \beta_{\text{leis}}(k))} \in [0, 1]$$
- Grupos com $\text{Clout} < 4\%$ tornam-se **Marginalizados**.
- Grupos com $\text{Clout} \ge 20\%$ tornam-se **Influentes** (seus bônus ou penalidades dobram).

**Aprovação ($A_g \in [-20, +20]$):**
$$A_g = \sum_{c \in \text{Categorias}} \text{Postura}(g, \text{LeiAtiva}_c) + \Delta_{\text{no\_governo}} + \frac{\text{Leais}_g - \text{Radicais}_g}{N_g} \cdot 10$$
- $A_g \ge +5$: **Leal** (concede bônus nacionais — ex.: Industriais leais aumentam $+10\%$ investimento; Sindicatos leais aumentam $+10\%$ throughput fabril).
- $A_g \le -10$: **Revoltoso / Insurgente** (bloqueia impostos/eficiência e pode iniciar crise revolucionária ou golpe).

---

### 5.3. Legitimidade e Eleições

O jogador escolhe quais Grupos de Interesse compõem a **Coalizão de Governo** ($G \subseteq \text{IGs}$):

$$\text{Legitimidade} = \text{clamp}\left(L_{\text{base}}(\text{lei\_governo}) + w_{\text{clout}} \sum_{g \in G} \text{Clout}(g) + w_{\text{votos}} \sum_{g \in G} \text{ParcelaVotos}(g) - \text{PenalidadeTamanho}(|G|) - \text{DivergenciaIdeologica}(G),\ 0,\ 100\right)$$

- **Faixas de Legitimidade**:
  - $0\text{–}24$ (*Governo Ilegítimo*): paralisia legislativa (impossível aprovar leis), $+2\%$ de novos radicais ao mês.
  - $25\text{–}49$ (*Governo Contestado*): $-25\%$ velocidade de reforma, leve crescimento de radicais.
  - $50\text{–}74$ (*Governo Legítimo*): funcionamento institucional normal.
  - $75\text{–}89$ (*Governo Justo*): $+25\%$ velocidade de reforma, $+1\%$ de novos leais ao mês.
  - $90\text{–}100$ (*Governo Íntegro*): $+50\%$ velocidade de reforma, $+2\%$ de novos leais ao mês.
- **Eleições**: Nos regimes com voto (`voto_censitario`, `voto_capacitario`, `sufragio_universal`), eleições ocorrem a cada **1.460 dias (4 anos)** (com campanha nos 180 dias anteriores), recalculando `ParcelaVotos(g)` a partir da demografia habilitada ao voto e do *momentum* da campanha.

---

### 5.4. Ideologias e Esferas de Influência (EUA vs. URSS)

A geopolítica global de `v1` orbita a competição bipolar entre **Washington (`USA`)** e **Moscou (`SOV`)**:

Cada país $c$ possui um **Vetor de Alinhamento Bipolar** $S_c \in [-100.0,\ +100.0]$:
- **$S_c \in [-100, -60]$**: **Esfera Ocidental Consolidada** (Aliado / Bloco do Dólar / futura OTAN).
- **$S_c \in (-60, -20]$**: **Inclinado ao Ocidente** (Parceiro Comercial / Receptor de Ajuda Externa tipo Plano Marshall).
- **$S_c \in (-20, +20)$**: **Não Alinhado / Neutro** (disputado por ambas as superpotências; futuro Movimento dos Não Alinhados).
- **$S_c \in [+20, +60)$**: **Inclinado ao Bloco Oriental** (Simpatizante / Acordos Bilaterais Soviéticos).
- **$S_c \in [+60, +100]$**: **Esfera Soviética Consolidada** (Estado Socialista Aliado / Satélite / futuro Comecon).

**Deriva Diária de Influência ($\Delta S_c$):**
$$\Delta S_c = \kappa \cdot \left(\text{AfinidadeRegime}(c) + \text{DependenciaComercial}(c) + \text{PressaoDiplomatica}(\text{SOV}) - \text{PressaoDiplomatica}(\text{USA}) - 0.05 \cdot S_c\right)$$
onde:
- `AfinidadeRegime(c)`: regimes com `partido_unico` + `economia_planificada` puxam fortemente para $+$; regimes com `sufragio_universal` + `livre_mercado`/`capitalismo_de_intervencao` puxam para $-$.
- **Ajuda Econômica Externa**: EUA e URSS podem destinar parte de seu orçamento fiscal para *Ajuda Econômica* (precursor do Plano Marshall e do Plano Molotov), injetando Pontos de Construção e divisas no país-alvo em troca de deslocamento acelerado de $S_c$ e abertura de mercado.
- **Tensão Mundial ($T_{\text{global}} \in [0, 100]$)**: começa em **$35$** em janeiro de 1945 (aliança antifascista já sob desconfiança em Yalta) e sobe conforme mudanças bruscas de regime, crises coloniais e disputas de esfera na Europa Central e Ásia Oriental.

---

## 6. Arquitetura de Software (Workspace Rust + Bevy)

```text
guerra-fria/
├── DESIGN.md                        # Este documento de especificação
├── Cargo.toml                       # Workspace Cargo
├── crates/
│   ├── simulacao/                   # Lib Rust pura (sem Bevy): tempo, economia, política, mapa, testes
│   ├── gerador_mapa/                # Binário CLI: lê Natural Earth 10m e gera PNG + RON + cenário 1945
│   └── guerra_fria/                 # Binário do jogo (Bevy + bevy_egui)
│       └── src/
│           ├── main.rs              # Bootstrap do App Bevy
│           └── plugins/
│               ├── map.rs           # MapPlugin: texturas, shader WGSL, câmera pan/zoom, clique
│               ├── time.rs          # TimePlugin: pausa (Espaço), velocidades 1-5, tick diário
│               ├── economy.rs       # EconomyPlugin: integração ECS do motor econômico
│               ├── politics.rs      # PoliticsPlugin: integração ECS do motor político
│               └── ui.rs            # UiPlugin: barra superior de tempo/data e painel egui de província
├── assets/
│   ├── map/
│   │   ├── provinces.png            # Bitmap 5632x2048 com cor RGB única por província
│   │   └── provinces.ron            # Metadados (id, nome, cor, centroide, área, vizinhos, país moderno)
│   └── shaders/
│       └── political_map.wgsl       # Shader GPU: lookup político + bordas de países e províncias
└── data/
    ├── raw/                         # Shapefiles brutos do Natural Earth (ignorado pelo git)
    └── scenario_1945/
        ├── countries_1945.ron       # Catálogo de países, impérios coloniais e ocupações de 1945
        ├── rules_1945.ron           # Regras de mapeamento adm0/adm1 -> tag de 1945
        └── province_owners_1945.ron # Tabela resolvida (provincia_id -> tag de 1945) para todo o globo
```
