# Sistema de Comércio Internacional e Tarifas (Estilo Victoria 3)

Este documento descreve a mecânica de comércio internacional implementada no motor econômico (`crates/sim_economia`) e na interface de usuário (`crates/guerra_fria`, atalho F3).

---

## 1. Visão Geral

O comércio internacional no jogo "Guerra Fria" (1945) é fundamentado no modelo econômico inspirado em Victoria 3, adaptado ao contexto geopolítico da Guerra Fria:
- **Fluxos Bilaterais de Mercadorias**: Rotas comerciais explícitas entre mercados por tipo de bem (importação ou exportação).
- **Tarifas e Protecionismo**: Controle tarifário por bem e nível geral de abertura comercial do mercado nacional.
- **Acordos Comerciais Bilaterais**: Redução mútua de 85% nas tarifas aduaneiras entre as nações signatárias.
- **Blocos Econômicos e COMECON**: Mercados do bloco soviético (URSS, Polônia, Tchecoslováquia, Romênia, Hungria, Bulgária, Alemanha Oriental, Albânia) operam inicialmente com comércio exterior restrito, abrindo-se para integração tarifária zero com a criação do Conselho para Assistência Econômica Mútua (COMECON).

---

## 2. Estruturas de Dados (`crates/sim_economia/src/entidades.rs`)

### Política Fiscal e Tarifária
```rust
pub struct PoliticaFiscal {
    // ...
    pub tarifas_importacao_bens: HashMap<String, f64>,
    pub tarifas_exportacao_bens: HashMap<String, f64>,
    pub nivel_abertura_mercado: f64, // 0.0 (autarquia) a 1.0 (livre-comércio)
}
```

### Acordos Comerciais e Blocos
```rust
pub struct Pais {
    // ...
    pub acordos_comerciais: Vec<String>,
}

pub struct BlocoComercial {
    pub id: String,
    pub nome: String,
    pub lider: String,
    pub membros: Vec<String>,
    pub fechado_ao_exterior: bool,
    pub isencao_tarifaria_interna: bool,
    pub ativo: bool,
}
```

---

## 3. Dinâmica de Mercado e Tarifas (`crates/sim_economia/src/motor/mercado.rs`)

### Resolução de Preços e Viabilidade
Uma rota de comércio é viável e rentável quando a margem de preço entre comprador e vendedor cobre o custo de transporte e as tarifas:
$$\text{Margem Efetiva} = P_{\text{importador}} \times (1 - \text{tarifa}_{\text{imp}}) - P_{\text{exportador}} \times (1 + \text{tarifa}_{\text{exp}}) - \text{Custo de Transporte}$$

1. **Desconto por Acordo Comercial**: Quando existe acordo bilateral entre origem e destino, as tarifas são multiplicadas por `0.15` (85% de desconto).
2. **Isenção Intra-Bloco COMECON**: Rotas entre membros do COMECON ativo têm tarifas zeradas.
3. **Bloqueio de Fronteiras**: Se um dos participantes pertence a um bloco fechado ao exterior (ex: Bloco Soviético antes da integração ou sem acordo bilateral), rotas comerciais externas não podem ser estabelecidas nem operadas.

---

## 4. Interface de Comércio F3 (`crates/guerra_fria/src/plugins/comercio_ui.rs`)

A interface é aberta com **F3** ou pelo botão **"Comércio (F3)"** no HUD superior do jogador.

### Abas da Interface:
1. **Rotas Ativas**: Lista detalhada de importações e exportações com volumes, receitas, custos tarifários, lucros e controles manuais (ajuste de nível/capacidade e botão de encerramento).
2. **Nova Rota**: Criação de rotas selecionando o bem, tipo (importar/exportar) e país parceiro com análise de preços, viabilidade e status diplomático/bloco.
3. **Tarifas e Abertura**:
   - Presets: *Livre-Câmbio* (abertura 100%, tarifas 0%), *Protecionismo* (abertura 60%, tarifas 20%), *Autarquia* (abertura 10%, tarifas 50%).
   - Sliders de nível de abertura e tarifas específicas por bem de consumo, intermediário e militar.
4. **Acordos e Blocos**:
   - Assinatura e cancelamento de acordos bilaterais de livre-comércio.
   - Painel do Bloco Econômico com estado dos membros e botão de ativação/integração do **COMECON** para os países da esfera socialista.
