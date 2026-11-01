# SISTEMA DE LOCALIZAÇÃO (i18n / l10n)
## Guerra Fria: 1945 — Arquitetura de Internacionalização em Rust + Bevy

> **Status:** Especificação de Arquitetura e Catálogo Base (pt-BR / en-US)  
> **Data Base:** 1º de Janeiro de 1945  
> **Módulo:** `enciclopedia` e interface global do usuário  

---

### 1. INTRODUÇÃO E OBJETIVOS

O sistema de localização de **Guerra Fria: 1945** foi concebido para atender a três requisitos primordiais:
1. **Fidelidade e Prioridade pt-BR:** O jogo é concebido e escrito primariamente em Português do Brasil, refletindo a historiografia, a cultura e a linguagem dos despachos da época (1945–1960).
2. **Prontidão para Tradução Internacional:** Toda string visível na interface possui uma chave unívoca em `snake_case` com pontuação semântica, pronta para ser traduzida para o Inglês (`en-US`) e outros idiomas no futuro.
3. **Poder de Gramática e Concordância:** Nomes de países e cargos possuem variações de gênero gramatical e artigo ("o Brasil", "a França", "os Estados Unidos", "o Primeiro-Ministro", "a Presidente") e formas plurais variáveis conforme variáveis numéricas.

---

### 2. COMPARAÇÃO ARQUITETURAL: FLUENT (.ftl) vs. RON

| Critério de Avaliação | Fluent (.ftl - Mozilla Project Fluent) | RON (Rust Object Notation) |
| :--- | :--- | :--- |
| **Tratamento de Pluralização** | **Excelente:** Suporte nativo a regras Unicode CLDR (zero, one, two, few, many, other) por idioma. | **Manual / Rígido:** Exige criar campos separados `singular` e `plural` na struct Rust. |
| **Gênero Gramatical e Concordância** | **Excelente:** Seletores internos por atributos (`[masculino]`, `[feminino]`, `[plural]`). | **Ineficiente:** Strings rígidas ou templates com interpolação simplista. |
| **Interpolação de Placeholders** | **Avançado:** Formatação de números, datas e moedas embutida na engine de texto. | **Básico:** Depende de macros de `format!` ou substituição com `str::replace`. |
| **Integração com Bevy** | **Nativa:** Suporte direto via crate `bevy_fluent`. | **Nativa:** Suporte direto via `serde` + `ron`. |
| **Amigabilidade para Tradutores** | **Padrão da Indústria:** Usado em navegadores, sistemas operacionais e ferramentas web (Crowdin/Weblate). | **Voltado a Devs:** Sintaxe de structs Rust pode confundir tradutores não técnicos. |

#### **Recomendação Oficial da Arquitetura:**
* **Para Dados Estruturados de Simulação (Entidades e Regras):** Manter arquivos **RON** (como `conceitos.ron`, `tutorial.ron`, `tooltips.ron`, `mensagens.ron`), onde cada entidade possui identificador único (`id: "padrao_de_vida"`) e campos em pt-BR de referência.
* **Para Localização em Tempo de Execução e Chaves da UI:** Utilizar o padrão **Project Fluent (.ftl)**, onde as chaves do RON são mapeadas 1:1 para arquivos `.ftl` organizados por idioma:
  * `assets/locales/pt-BR/enciclopedia.ftl`
  * `assets/locales/en-US/enciclopedia.ftl`

---

### 3. CONVENÇÃO DE NOMENCLATURA DE CHAVES

Todas as chaves do sistema seguem a estrutura hierárquica delimitada por pontos e identificadores `snake_case` ASCII:

```text
[modulo].[submodulo].[entidade_id].[atributo]
```

Exemplos práticos:
* `tutorial.passo_01_bem_vindo_a_1945.titulo`
* `tutorial.passo_01_bem_vindo_a_1945.texto`
* `conceito.padrao_de_vida.nome`
* `conceito.padrao_de_vida.definicao_mecanica`
* `tooltip.topbar_reservas_tesouro.titulo`
* `tooltip.topbar_reservas_tesouro.descricao`
* `mensagem.msg_lei_aprovada.titulo`
* `mensagem.msg_lei_aprovada.corpo`

---

### 4. EXEMPLO DE REGRAS EM FLUENT (.ftl)

No formato Fluent, a concordância com o artigo definido do país e a variação plural de valores monetários e semanas de obra é tratada de forma elegante:

```fluent
# assets/locales/pt-BR/mensagens.ftl

msg-lei-aprovada-titulo = Reforma Legal: { $pais_nome }
msg-lei-aprovada-corpo = 
    O parlamento d{ $artigo_pais ->
       *[de] e
        [do] o
        [da] a
        [dos] os
        [das] as
    } { $pais_nome } concluiu a tramitação e promulgou oficialmente a lei "{ $lei_nome }".

msg-predio-inaugurado-corpo =
    A construção de "{ $predio_nome }" na província de { $provincia_nome } ({ $pais_tag }) foi concluída.
    { $semanas ->
        [one] A obra durou apenas 1 semana.
       *[other] A obra durou { $semanas } semanas.
    }
```

E no correspondente em Inglês:

```fluent
# assets/locales/en-US/mensagens.ftl

msg-lei-aprovada-titulo = Legal Reform: { $pais_nome }
msg-lei-aprovada-corpo = 
    The parliament of { $pais_nome } has concluded deliberations and formally enacted the law "{ $lei_nome }".

msg-predio-inaugurado-corpo =
    The construction of "{ $predio_nome }" in the province of { $provincia_nome } ({ $pais_tag }) has been completed.
    { $semanas ->
        [one] Construction took only 1 week.
       *[other] Construction took { $semanas } weeks.
    }
```

---

### 5. CATÁLOGO COMPLETO DE CHAVES E TRADUÇÃO (pt-BR / en-US)

Abaixo encontra-se a tabela mestre de equivalência entre as chaves do sistema, o texto canônico em pt-BR e o texto traduzido para inglês (en-US) para os elementos de interface, HUD, comandos e menus:

| Chave de Localização | Texto Canônico (pt-BR) | Tradução Pronta (en-US) |
| :--- | :--- | :--- |
| `hud.topbar.pais` | País e Soberania | Nation & Sovereignty |
| `hud.topbar.lider` | Chefe de Governo | Head of Government |
| `hud.topbar.tesouro` | Reservas do Tesouro | Treasury Reserves |
| `hud.topbar.balanco_diario` | Balanço Diário | Daily Fiscal Balance |
| `hud.topbar.padrao_ouro` | Padrão Ouro / Câmbio | Gold Standard / Forex |
| `hud.topbar.inflacao` | Inflação Anual | Annual Inflation |
| `hud.topbar.legitimidade` | Legitimidade Governamental | Government Legitimacy |
| `hud.topbar.estabilidade` | Estabilidade Nacional | National Stability |
| `hud.topbar.prestigio` | Prestígio Internacional | International Prestige |
| `hud.topbar.doomsday_clock` | Relógio do Juízo Final | Doomsday Clock |
| `hud.topbar.data` | Data da Simulação | Simulation Date |
| `hud.topbar.pausa` | Pausar / Continuar | Pause / Resume |
| `hud.topbar.velocidade` | Velocidade do Jogo | Game Speed |
| `hud.minimapa.titulo` | Radar Tático do Globo | Tactical Global Radar |
| `hud.minimapa.zoom_in` | Aproximar Visão | Zoom In |
| `hud.minimapa.zoom_out` | Afastar Visão | Zoom Out |
| `hud.minimapa.capital` | Focar na Capital | Focus on Capital |
| `hud.minimapa.norte` | Redefinir para o Norte | Reset to North |
| `hud.mapmode.politico` | Modo Político | Political Map Mode |
| `hud.mapmode.ideologico` | Modo Ideológico | Ideological Map Mode |
| `hud.mapmode.economico` | Modo Econômico | Economic Map Mode |
| `hud.mapmode.esferas` | Modo de Esferas | Spheres of Influence |
| `hud.mapmode.tensao` | Modo de Tensão | Tension & Unrest Mode |
| `hud.mapmode.recursos` | Modo de Recursos | Strategic Resources |
| `hud.telegramas.titulo` | Despachos Diplomáticos | Diplomatic Dispatches |
| `hud.telegramas.minimizar` | Recolher Painel | Minimize Drawer |
| `painel.orcamento.titulo` | Livro-Razão Fiscal e Orçamento | Fiscal Ledger & Budget |
| `painel.orcamento.receitas` | Receitas Tributárias | Tax Revenues |
| `painel.orcamento.despesas` | Despesas Governamentais | Public Expenditures |
| `painel.orcamento.divida` | Dívida Pública e Títulos | Public Debt & Securities |
| `painel.mercado.titulo` | Mercado Nacional de Bens | National Goods Market |
| `painel.mercado.oferta` | Oferta Agregada | Aggregate Supply |
| `painel.mercado.demanda` | Demanda Agregada | Aggregate Demand |
| `painel.mercado.preco_base` | Preço de Referência | Baseline Price |
| `painel.mercado.preco_atual` | Preço de Mercado | Current Market Price |
| `painel.construcao.titulo` | Fila Nacional de Construção | National Construction Queue |
| `painel.construcao.pontos` | Pontos de Capacidade | Construction Capacity Points |
| `painel.construcao.pausar` | Suspender Obras | Suspend Construction |
| `painel.politica.titulo` | Gabinete e Forças Políticas | Cabinet & Political Forces |
| `painel.politica.forca` | Força Política (Clout) | Political Clout |
| `painel.politica.gabinete` | Membros da Coalizão | Coalition Members |
| `painel.politica.oposicao` | Grupos na Oposição | Opposition Groups |
| `painel.leis.titulo` | Tramitação de Leis | Legislative Docket |
| `painel.leis.avancar` | Promulgar Reforma | Enact Reform |
| `painel.leis.cancelar` | Retirar de Pauta | Withdraw Bill |
| `painel.populacao.titulo` | Censo Demográfico Provincial | Provincial Demographics Census |
| `painel.populacao.padrao_vida` | Padrão de Vida Médio | Average Standard of Living |
| `painel.populacao.radicais` | Cidadãos Radicalizados | Radicalized Citizens |
| `painel.populacao.legalistas` | Cidadãos Legalistas | Loyalist Citizens |
| `painel.diplomacia.titulo` | Chancelaria de Relações Exteriores | Foreign Relations Chancery |
| `painel.diplomacia.esfera` | Incorporar à Esfera | Add to Sphere |
| `painel.diplomacia.acordo` | Acordo Comercial | Trade Agreement |
| `painel.diplomacia.alianca` | Aliança Militar Defensiva | Defensive Military Alliance |

---

### 6. CÓDIGO RUST DE INTEGRAÇÃO COM BEVY

Abaixo está o exemplo idiomático de recurso e sistema de consulta de localização para Bevy:

```rust
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Debug, Clone)]
pub struct LocalizationManager {
    pub current_locale: String,
    pub fallback_locale: String,
    strings: HashMap<String, String>,
}

impl LocalizationManager {
    pub fn new(locale: &str) -> Self {
        let mut manager = Self {
            current_locale: locale.to_string(),
            fallback_locale: "pt-BR".to_string(),
            strings: HashMap::new(),
        };
        manager.load_locale(locale);
        manager
    }

    pub fn load_locale(&mut self, _locale: &str) {
        // Carrega arquivos .ftl ou dicionário RON em tempo de execução
    }

    /// Retorna a string localizada ou a chave entre colchetes caso ausente
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.strings.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// Retorna a string com interpolação de argumentos dinâmicos
    pub fn format(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut template = self.get(key).to_string();
        for (var, val) in args {
            let placeholder = format!("{{{}}}", var);
            template = template.replace(&placeholder, val);
        }
        template
    }
}
```

---

### 7. GARANTIA DE AUSÊNCIA DE CHAVES DUPLICADAS

Para assegurar integridade referencial total:
* O script de validação `validar.py` percorre todos os arquivos `.ron` e compila o conjunto de chaves.
* Se qualquer `id` em `tutorial.ron`, `conceitos.ron`, `tooltips.ron` ou `mensagens.ron` aparecer mais de uma vez (mesmo entre arquivos distintos ou dentro da mesma lista), o processo de validação emite erro fatal e aborta a compilação.
