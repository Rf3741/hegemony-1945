# sim_economia — Motor de Simulação Econômica Victoria 3 (Guerra Fria 1945)

`sim_economia` é um crate de biblioteca Rust pura (sem dependência de engine gráfica ou Bevy) que implementa o motor de simulação socioeconômica profunda inspirado na arquitetura de *Victoria 3*, adaptado para o contexto histórico da Guerra Fria (a partir de 01/01/1945).

O motor roda em tempo discreto determinístico (tick diário com suporte a cadência de ajuste estrutural semanal), operando sobre cadeias produtivas complexas, dinâmica de classes e estratos sociais, mercado de trabalho, elasticidade de consumo, comércio exterior e finanças públicas de governos soberanos.

---

## 1. Convenções de Identificadores e Esquema RON

Todos os dados de configuração estática e inicialização de cenários são carregados e exportados no formato **RON (Rust Object Notation)** com serialização via `serde` + `ron`.

### Regras de Nomenclatura e Tipagem
- **Identificadores (`id`)**: estritamente em ASCII `snake_case` minúsculo (ex.: `graos`, `trabalhadores_fabris`, `siderurgica_moderna`, `uniao_sovietica`, `vale_do_paraiba`). Caracteres permitidos: `[a-z0-9_]`.
- **Nomes de exibição (`nome`)**: formato legível em português brasileiro (`pt-BR`), com acentuação e pontuação adequadas (ex.: `"Grãos e Cereais"`, `"Operários Fabris"`, `"Siderúrgica"`, `"União Soviética"`).
- **Datas históricas**: tuplas com validação gregoriana `(ano: 1945, mes: 1, dia: 1)`.
- **Campos numéricos**: `f64` para fluxos contínuos e estoques econômicos (população, preços, saldos, reservas); `u32` para contadores discretos (nível de prédios, limites de terra arável e recursos).

---

## 2. Esquema dos Arquivos RON

Os arquivos do catálogo econômico e do cenário situam-se em subpastas ou no diretório de dados (ex.: `tests/fixtures/`):

### 2.1. `bens.ron` (`Vec<Bem>`)
Define os bens comercializáveis e serviços da economia mundial.

```ron
[
    (
        id: "graos",
        nome: "Grãos e Cereais",
        categoria: AlimentoBasico, // AlimentoBasico | MateriaPrima | Energia | Industrial | Consumo | Luxo | Militar | LocalNaoComercializavel
        preco_base: 20.0,
        comercializavel: true,
        substituicoes: [
            (
                necessidade: AlimentacaoBasica,
                peso: 1.4,
                fracao_maxima: 0.85,
                fracao_minima: 0.15,
            ),
        ],
    ),
]
```

### 2.2. `tipos_pop.ron` (`Vec<TipoPop>`)
Define as profissões / classes sociais da população.

```ron
[
    (
        id: "operarios",
        nome: "Operários Fabris",
        estrato: Baixo, // Baixo | Medio | Alto
        multiplicador_salario: 1.0,
        alfabetizacao_minima: 0.15,
        subsistencia: false,
        recebe_dividendos_capitalista: false,
        recebe_dividendos_aristocrata: false,
    ),
    (
        id: "capitalistas",
        nome: "Capitalistas Industriais",
        estrato: Alto,
        multiplicador_salario: 3.5,
        alfabetizacao_minima: 0.70,
        subsistencia: false,
        recebe_dividendos_capitalista: true,
        recebe_dividendos_aristocrata: false,
    ),
]
```

### 2.3. `metodos_producao.ron` (`Vec<MetodoProducao>`)
Define técnicas produtivas, métodos de automação e regimes de propriedade para prédios.

```ron
[
    (
        id: "siderurgia_alto_forno_moderno",
        nome: "Alto-Forno Contínuo Moderno",
        slot: ProducaoPrincipal, // ProducaoPrincipal | Automacao | Propriedade | Secundario
        insumos: [
            (bem_id: "ferro", quantidade_diaria: 1.2),
            (bem_id: "carvao", quantidade_diaria: 1.0),
        ],
        produtos: [
            (bem_id: "aco", quantidade_diaria: 1.5),
        ],
        empregos: [
            (tipo_pop_id: "operarios", delta_vagas: 4500),
            (tipo_pop_id: "engenheiros", delta_vagas: 400),
            (tipo_pop_id: "burocratas", delta_vagas: 100),
        ],
        regime_propriedade: None, // Some(PrivadoCapitalista | Aristocratico | Estatal | Cooperativo) | None
        pontos_construcao_gerados: 0.0,
        burocracia_gerada: 0.0,
        forca_militar_gerada: 0.0,
        infraestrutura_gerada: 0.0,
    ),
]
```

### 2.4. `tipos_predio.ron` (`Vec<TipoPredio>`)
Define os tipos de edificações e instalações produtivas.

```ron
[
    (
        id: "fabrica_aco",
        nome: "Siderúrgica",
        categoria: Industrial, // RuralAgricola | Extrativo | Industrial | Infraestrutura | Construcao | Governamental | Militar
        recurso_necessario_id: None,
        usa_terra_aravel: false,
        custo_pontos_construcao: 450.0,
        financiado_pelo_governo: false,
        metodos_padrao: [
            "siderurgia_alto_forno_moderno",
            "propriedade_privada_capitalista",
        ],
        metodos_permitidos: [
            "siderurgia_alto_forno_moderno",
            "siderurgia_bessemer_legado",
            "propriedade_privada_capitalista",
            "propriedade_estatal",
        ],
    ),
]
```

### 2.5. `leis_economicas.ron` (`Vec<LeiEconomica>`)
Define o quadro institucional e jurídico das nações.

```ron
[
    (
        id: "economia_de_mercado",
        nome: "Economia de Mercado Regulada",
        grupo: SistemaEconomico, // SistemaEconomico | Tributacao | Comercio
        multiplicador_eficiencia_construcao: 1.10,
        multiplicador_fundo_investimento_privado: 1.25,
        multiplicador_volume_comercio: 1.20,
        exige_propriedade_estatal: false,
    ),
    (
        id: "imposto_renda_proporcional",
        nome: "Tributação sobre Renda e Lucro",
        grupo: Tributacao,
        multiplicador_eficiencia_construcao: 1.0,
        multiplicador_fundo_investimento_privado: 1.0,
        multiplicador_volume_comercio: 1.0,
        exige_propriedade_estatal: false,
    ),
]
```

### 2.6. `mercados.ron` (`Vec<Mercado>`)
Define praças de precificação onde ordens de compra e venda são agregadas.

```ron
[
    (
        id: "mercado_norte_americano",
        nome: "Mercado Comum Norte-Americano",
        pais_lider_id: "estados_unidos",
    ),
]
```

### 2.7. `paises.ron` (`Vec<Pais>`)
Representa os estados soberanos, finanças e políticas públicas.

```ron
[
    (
        id: "estados_unidos",
        nome: "Estados Unidos da América",
        tag: Some("USA"),
        mercado_id: "mercado_norte_americano",
        culturas_primarias: ["anglo_americana"],
        religiao_oficial: "protestante",
        politica_fiscal: (
            lei_sistema_economico_id: "economia_de_mercado",
            lei_tributaria_id: "imposto_renda_proporcional",
            lei_comercial_id: "livre_cambio",
            aliquota_imposto_renda_baixo: 0.10,
            aliquota_imposto_renda_medio: 0.16,
            aliquota_imposto_renda_alto: 0.25,
            aliquota_imposto_dividendos: 0.18,
            imposto_sobre_terra_por_campones: 0.0,
            imposto_sobre_terra_por_nivel_rural: 5.0,
            aliquota_consumo_geral: 0.05,
            tarifa_importacao: 0.06,
            tarifa_exportacao: 0.0,
            multiplicador_salario_governo: 1.0,
            multiplicador_salario_militar: 1.0,
            nivel_investimento_educacao: 3,
        ),
        pontos_construcao_base: 30.0,
        orcamento: (
            tesouro: 1500000.0,
            divida: 250000.0,
            taxa_juros_anual_base: 0.035,
        ),
        fundo_investimento_privado: 450000.0,
        fila_construcao: [],
    ),
]
```

### 2.8. `estados.ron` (`Vec<Estado>`) e `provincias.ron` (`Vec<Provincia>`)
Define a malha geográfica, infraestrutura, população inicial e prédios existentes.

```ron
// estados.ron
[
    (
        id: "estado_nova_york",
        nome: "Nova York",
        pais_id: "estados_unidos",
        incorporado: true,
        infraestrutura_bonus: 50.0,
    ),
]

// provincias.ron
[
    (
        id: "prov_manhattan",
        nome: "Manhattan",
        codigo_externo: Some("USA-3521"),
        estado_id: "estado_nova_york",
        pais_id: "estados_unidos",
        terra_aravel_total: 10,
        limites_recursos: {
            "recurso_porto": 5,
        },
        infraestrutura_base: 40.0,
        pops: [
            (
                tipo_pop_id: "operarios",
                cultura: "anglo_americana",
                religiao: "protestante",
                populacao: 250000.0,
                empregados: 240000.0,
                riqueza_reservas: 1200000.0,
                padrao_de_vida: 14.5,
                alfabetizacao: 0.88,
            ),
        ],
        predios: [
            (
                tipo_predio_id: "fabrica_aco",
                nivel: 5,
                metodos_ativos: ["siderurgia_alto_forno_moderno", "propriedade_privada_capitalista"],
                salario_base: 4.8,
                reservas_caixa: 15000.0,
                subsidiado: false,
            ),
        ],
    ),
]
```

### 2.9. `rotas_comerciais.ron` (`Vec<RotaComercial>`)
Define o intercâmbio de mercadorias entre mercados autônomos.

```ron
[
    (
        id: "rota_carvao_eua_europa",
        mercado_origem_id: "mercado_norte_americano",
        mercado_destino_id: "mercado_europeu",
        pais_operador_id: "estados_unidos",
        bem_id: "carvao",
        volume_atual: 50.0,
        volume_maximo: 200.0,
        automatica: true,
    ),
]
```

---

## 3. O Ciclo Econômico: As 9 Mecânicas da Simulação

A simulação é executada no método `MundoEconomico::executar_tick_diario()`, implementando 9 fases interligadas:

1. **Produção Física dos Prédios**:
   - Calculada a partir do nível, taxa de ocupação de vagas (`r_emp`), acesso ao mercado (`acesso`), economia de escala (`+1%` por nível acima de 1 até 25) e disponibilidade física de insumos no mercado (`fator_insumos`).
   - Populações de subsistência (camponeses) em terras aráveis não ocupadas produzem alimentos básicos de subsistência.
   - Gera pontos de construção de setores de obras, burocracia governamental e projeção de força militar.

2. **Formação de Preços de Mercado**:
   - Consolidação de ordens de venda (oferta) e ordens de compra (demanda) em cada mercado.
   - Aplicação das rotas comerciais internacionais com ajuste de volume por lucratividade.
   - Precificação segundo a fórmula canônica Victoria 3 amortecida:
     $$\text{alvo} = \text{base} \times \operatorname{clamp}\left(1 + 0.75 \times \frac{D - O}{\max(O, D, V_{\min})}, 0.25, 1.75\right)$$
     com suavização exponencial diária configurável para eliminar ruídos transitórios.

3. **Salários, Lucros e Finanças Operacionais**:
   - Apuração do valor adicionado ($\text{receita} - \text{insumos}$).
   - Pagamento de folha salarial escalonada por multiplicador profissional.
   - Cobertura de déficits operacionais por reservas de caixa do prédio ou subsídios estatais; formação de reservas de caixa retendo até 35% do lucro líquido.
   - Distribuição de excedentes em dividendos aos proprietários conforme o regime societário: capitalistas (setor urbano), aristocratas (setor agrário), tesouro nacional (empresas públicas) ou cooperativa de operários.

4. **Consumo das Famílias e Substituição de Bens**:
   - Estratificação em 6 necessidades universais de consumo (Alimentação Básica, Vestuário, Energia/Calefação, Serviços Urbanos, Mobilidade/Comunicação, Artigos de Luxo).
   - Aplicação da Lei de Engel: conforme o padrão de vida sobe, a fatia gasta em alimentos decresce e a procura por serviços e artigos de luxo expande.
   - Substituição elástica com base na razão de preço efetivo com tributação:
     $$u_g \propto \text{peso}_g \times \text{fator\_oferta} \times \left(\frac{P_g \cdot (1 + \tau_g)}{P_g^0}\right)^{-\eta}$$

5. **Padrão de Vida (SoL 1..99) e Alfabetização**:
   - Razão entre consumo real auferido e o custo de vida base da faixa de SoL.
   - Convergência diária suave em direção ao nível de bem-estar atingido.
   - A alfabetização evolui em direção ao teto educacional fornecido pelo padrão de vida das famílias e pelo investimento escolar do Estado.

6. **Transição Demográfica e Migração**:
   - Curva em U-invertido para crescimento vegetativo: SoL extremo baixo gera mortalidade; SoL intermediário gera explosão demográfica; SoL elevado e alta alfabetização estabilizam o crescimento natural (transição demográfica moderna).
   - Migração interna interprovincial impulsionada por atratividade (diferencial de SoL médio ponderado e vagas abertas no mercado de trabalho formal).

7. **Mercado de Trabalho Dinâmico**:
   - Prédios lucrativos abrem vagas e contratam a taxas diárias controladas; prédios em prejuízo demitem trabalhadores caso esgotem as reservas de caixa.
   - Promoção social: camponeses e operários de baixa qualificação são promovidos a cargos técnicos e especializados caso satisfaçam os requisitos mínimos de alfabetização.
   - Mecanismo de ajuste salarial de mercado: escassez prolongada de mão de obra eleva o salário-base; perdas prolongadas reduzem os salários.

8. **Orçamento Público e Dívida Soberana**:
   - Arrecadação de tributos diretos (renda baixa/média/alta), lucros/dividendos, impostos fundiários e impostos de consumo (geral e seletivo).
   - Gastos com manutenção burocrática, prontidão militar, canteiros de obras públicas, subsídios corporativos e serviço da dívida pública.
   - Limite de crédito nacional lastreado no PIB soberano; juros progressivos por alavancagem e tratamento de moratória/calote.

9. **Fila de Construção Civil**:
   - Geração de pontos de construção a partir da capacidade base nacional somada a prédios do setor de construção.
   - Rateio de pontos entre obras públicas e investimentos privados (financiados pelo acúmulo de dividendos no Fundo de Investimento Privado).
   - Conclusão automática de obras com materialização de novos níveis de prédios na infraestrutura física das províncias.

---

## 4. Integração com Outros Módulos do Projeto

`sim_economia` foi projetado para interoperar de forma desacoplada com as demais pastas de `/home/rafael/claude/guerra-fria-staging`:

- **`nacoes/`**: Fornece os códigos e tags ISO/históricos das nações (ex.: `USA`, `SOV`, `BRA`, `GBR`, `FRA`), preenchidos no campo `Pais::tag` e `Pais::id`.
- **`provincias_1945/` / `regioes_1945/`**: Mapeamento geográfico do mundo real de 1945 com mais de 3.000 polígonos e províncias, integrando-se via `Provincia::codigo_externo` (ex.: `"USA-3521"`, `"SOV-1042"`).
- **Sem dependência de Bevy**: O crate pode ser compilado como biblioteca nativa, rodado como CLI autônomo para benchmarks ou acoplado como um Resource em sistemas Bevy no projeto principal via pontes de dados serializáveis.

## Desempenho (benchmark)

`cargo bench` (bench `tick_mundo`) simula 150 países, 750 estados, 3200 províncias, 60 mercados, ~13 mil prédios e ~22 mil grupos de pop.
Medição em máquina compartilhada (load ~4,7): tick diário médio ~13,8 ms (p95 ~17,5 ms), distribuído de forma uniforme entre as fases (produção, distribuição, consumo, demografia, trabalho). A meta de < 5 ms **ainda não foi atingida**; o bench falha de propósito (assert) enquanto isso. Próximos passos: paralelizar por país/mercado (rayon), evitar alocações por tick e usar layout SoA nos grupos de pop.
