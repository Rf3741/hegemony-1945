# Documentação da UI Refinada — Guerra Fria (01/01/1945)

> **Módulo:** `ui_refinada`  
> **Tecnologia:** Rust + Bevy Engine + `bevy_egui` + `egui_plot`  
> **Direção de Arte:** Gabinete de Guerra, Documentos Confidenciais, Papel Envelhecido & Teletipo Industrial (1945–1960).  
> **Idioma:** Português do Brasil (`pt-BR`).

---

## 1. Visão Geral da Arquitetura

O crate `ui_refinada` implementa de forma completa, nativa e funcional a interface de usuário planejada nos documentos `design/1_identidade`, `design/2_telas` e `design/5_hud_menu`.

A arquitetura adota o padrão **MVVM (Model - View - ViewModel)**:
- **View-Models Desacoplados (`src/view_model/`):** Estruturas de dados limpas com suporte a `serde` que representam o estado visual das telas sem depender do motor de física ou banco de dados relacional.
- **Widgets Especializados (`src/widgets/`):**
  - `pie_chart`: Renderizador de gráficos de pizza com tesselamento vetorial e realce angular sob cursor (*hover*).
  - `nested_tooltip`: Sistema analítico de tooltips aninhados no estilo aclamado de *Victoria 3*, decompondo fórmulas e fatores aditivos/subtrativos.
  - `stamps`: Carimbos militares ("ULTRASSECRETO", "CONFIDENCIAL", "URGENTE") com contorno industrial.
  - `meter`: Medidor de nível DEFCON e barras industriais retas.
- **Telas Especializadas (`src/screens/`):** Implementações completas dos painéis e janelas correspondentes aos atalhos `ESC` e `F1` a `F8`.
- **Plugin Reutilizável (`UiPlugin` em `src/plugin.rs`):** Plugin de Bevy que pode ser acoplado diretamente a qualquer executável do jogo.

---

## 2. Mapa de Telas e Teclas de Atalho

| Tecla | Tela | Descrição |
| :---: | :--- | :--- |
| **ESC** | **Mapa Geral com HUD** | Viewport do mapa com HUD superior (48px) e gaveta de telegramas datilografados. |
| **F1** | **Painel do País** | Dossiê de superpotência: Roosevelt/Truman, fluxo do tesouro, IC civil/militar e alianças. |
| **F2** | **Painel do Estado** | Gestão de província (Pensilvânia/Pittsburgh): **Gráficos de Pizza de Pops** (Profissão, Cultura, Religião), prédios e jazidas. |
| **F3** | **Tela do Mercado** | Cotações de bens com desvio %, ações comerciais e **Gráfico de Histórico de Preços** com `egui_plot`. |
| **F4** | **Construção & Obras** | Fila nacional com reordenação (Subir/Descer), pausar, cancelar e catálogo de instalações. |
| **F5** | **Orçamento & Tesouro** | **Sliders interativos** de alíquotas tributárias e custeio público com cálculo de saldo em tempo real. |
| **F6** | **Política & Leis** | 7 categorias institucionais de leis ativas, projeto de lei em tramitação com probabilidades e 11 Grupos de Interesse (IIGG). |
| **F7** | **Diplomacia** | Comparativo bipolar (EUA x URSS), termômetro DEFCON e iniciativas diplomáticas. |
| **F8** | **Ranking das Potências** | Classificação das 8 grandes potências de 1945 com fórmula de prestígio ponderada. |
| **ESPAÇO** | **Pausar / Continuar** | Alterna entre pausa e execução do relógio de simulação. |
| **1 a 5** | **Velocidade** | Ajusta a velocidade da simulação de 1x a 5x. |
| **E** | **Evento Modal** | Abre a simulação da Conferência de Yalta (1945) com 3 ramificações de escolha. |

---

## 3. Guia de Integração no Jogo Principal

### 3.1. Adicionar Dependência no `Cargo.toml` do jogo:

```toml
[dependencies]
ui_refinada = { path = "../ui_refinada" }
bevy = "0.20.0"
bevy_egui = "0.43.1"
```

### 3.2. Registrar o Plugin no `App` do Bevy:

```rust
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use ui_refinada::UiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(UiPlugin)
        .run();
}
```

### 3.3. Atualizar Dados a partir da Simulação ECS:

Como os View-Models são registrados como recursos normais do Bevy (`Resource`), qualquer sistema do motor pode injetá-los e atualizá-los a cada tick diário:

```rust
use bevy::prelude::*;
use ui_refinada::view_model::TopBarViewModel;

pub fn atualizar_top_bar_com_dados_reais(
    mut top_bar: ResMut<TopBarViewModel>,
    // query ou recurso da simulação econômica
) {
    // Atualiza apenas os números calculados pelo motor de simulação:
    top_bar.tesouro_milhoes = 18520.4;
    top_bar.balanco_diario = 43100.0;
}
```

---

## 4. Estrutura dos Arquivos da Crate

```
ui_refinada/
├── Cargo.toml                  # Dependências e perfis de build
├── UI.md                       # Este documento
├── tests/
│   └── testes_ui.rs            # Testes unitários automatizados
├── assets/
│   └── fonts/                  # Fontes históricas locais (Barlow, Courier Prime, Oswald, Special Elite)
└── src/
    ├── lib.rs                  # Entrada da biblioteca e re-exportações
    ├── main.rs                 # Executável autônomo com Bevy App interativo
    ├── plugin.rs               # UiPlugin para o loop ECS do Bevy
    ├── mock_data.rs            # Cenários históricos de 1945 (EUA e URSS)
    ├── theme/                  # Paleta de cores e estilo visual de 1945
    │   ├── mod.rs
    │   ├── paleta.rs           # Tokens de cores (superfícies, papel, status, geopolítica)
    │   └── estilo.rs           # Configuração de egui::Visuals e egui::Style
    ├── view_model/             # View-Models desacoplados
    │   ├── mod.rs
    │   ├── top_bar.rs          # HUD superior
    │   ├── pais.rs             # F1: País
    │   ├── estado.rs           # F2: Estado e Pops
    │   ├── mercado.rs          # F3: Mercado e cotações
    │   ├── construcao.rs       # F4: Fila de obras
    │   ├── orcamento.rs        # F5: Sliders do orçamento
    │   ├── politica.rs         # F6: Leis e IIGG
    │   ├── diplomacia.rs       # F7: Relações bipolares
    │   ├── ranking.rs          # F8: Classificação global
    │   ├── notificacoes.rs     # Despachos por telegrama
    │   └── eventos.rs          # Modal de Yalta
    ├── widgets/                # Widgets personalizados egui
    │   ├── mod.rs
    │   ├── pie_chart.rs        # Gráficos de pizza com legenda
    │   ├── nested_tooltip.rs   # Tooltips analíticos Victoria 3
    │   ├── stamps.rs           # Carimbos confidenciais
    │   └── meter.rs            # DEFCON e medidores
    └── screens/                # Renderizadores de tela egui
        ├── mod.rs
        ├── hud_top_bar.rs
        ├── tela_pais.rs
        ├── tela_estado.rs
        ├── tela_mercado.rs
        ├── tela_construcao.rs
        ├── tela_orcamento.rs
        ├── tela_politica.rs
        ├── tela_diplomacia.rs
        ├── tela_ranking.rs
        ├── painel_notificacoes.rs
        └── popup_evento.rs
```

## Integração no workspace (T14)

`ui_refinada` é um crate de biblioteca em `crates/ui_refinada` (deps via `[workspace.dependencies]`: `bevy_egui`, `egui` 0.36, `egui_plot` 0.37; uma única versão de `egui`). Exporta `UiPlugin` e os view-models (`ui_refinada::view_model`).

- Testes: `cargo test -p ui_refinada`
- Demo com dados mock: `cargo run -p ui_refinada --features demo --example demo`
- Fontes embutidas via `include_bytes!` a partir de `crates/ui_refinada/assets/fonts` (sem caminhos externos).
- Uso no cliente (T18): `app.add_plugins(EguiPlugin::default()).add_plugins(UiPlugin)` e preencher os view-models por um adaptador `MundoSim -> ViewModels`; ainda NÃO ligado ao `guerra_fria`.
