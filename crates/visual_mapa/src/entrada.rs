//! Interface de dados de entrada do `VisualMapaPlugin`.
//!
//! O plugin NÃO conhece a simulação: ele apenas desenha o que está em [`EntradaVisual`] e em
//! [`AnoVisual`]. Quem integra (`guerra_fria`/T19, alimentado por `sim_territorio`) escreve
//! esses recursos; o plugin faz o resto (spawn, crescimento animado, era, LOD).
//!
//! ## Contrato
//! - `posicao` é `(longitude, latitude)` em graus; o plugin projeta com o Miller do `gf_mapa`.
//! - `id` é a chave estável (ex.: `adm1_code` ou id da cidade). Entradas novas criam entidades;
//!   entradas existentes atualizam nível/produção; entradas ausentes NÃO são removidas
//!   (marque `ativa = false` para escondê-las).
//! - Cidade: `nivel` 1..=8 (fracionário ok; usa-se o piso). Quando sobe, os prédios "brotam".
//! - Instalação: `tipo` livre (`fabrica`, `mina`, `poco_petroleo`, `usina`, `porto` ou os tipos
//!   do atlas como `siderurgica`, `refinaria`, `usina_nuclear`...); `producao` 0..=1 comanda a fumaça.
//! - Era visual: global por [`AnoVisual`] (1945-1954 = 0 ... 1985+ = 4) ou, por entrada,
//!   `era: Some(0..=4)` (ex.: a era local do território, `GerenciadorEras` do jogo).
//! - Mude o recurso só quando algo mudar: os sistemas de sincronia rodam em `is_changed()`.
//!
//! ```ignore
//! fn alimentar(mut entrada: ResMut<EntradaVisual>, mut ano: ResMut<AnoVisual>, sim: Res<MundoSim>) {
//!     ano.0 = sim.relogio.data.ano as f32 + sim.relogio.data.fracao_do_ano();
//!     entrada.cidades = sim.territorio.cidades().map(|c| CidadeEntrada { id: c.id.clone(), nivel: c.nivel as f32, ..default() }).collect();
//! }
//! ```

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Ano visual contínuo (ex.: 1962.4), derivado da data do relógio da simulação.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct AnoVisual(pub f32);

impl Default for AnoVisual {
    fn default() -> Self {
        Self(1945.0)
    }
}

/// Estilo arquitetônico do atlas por bloco geopolítico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BlocoVisual {
    #[default]
    Ocidental,
    Sovietica,
    TerceiroMundo,
}

impl BlocoVisual {
    /// Chave usada em `atlas.ron` (campo `bloco`).
    pub fn chave_atlas(self) -> &'static str {
        match self {
            Self::Ocidental => "ocidental",
            Self::Sovietica => "sovietica",
            Self::TerceiroMundo => "terceiro_mundo",
        }
    }

    /// Bloco aproximado a partir da tag do país (usado quando a entrada não informa `bloco`).
    pub fn de_pais(tag: &str) -> Self {
        match tag {
            "SOV" | "UKR" | "BLR" | "KAZ" | "UZB" | "GEO" | "ARM" | "AZE" | "LTU" | "LVA" | "EST"
            | "POL" | "CZE" | "SVK" | "HUN" | "ROU" | "BGR" | "ALB" | "YUG" | "SRB" | "CCP" | "CHI"
            | "PRK" | "MNG" | "VNM" | "CUB" | "GDR" | "DDR" => Self::Sovietica,
            "USA" | "CAN" | "GBR" | "FRA" | "GER" | "ITA" | "ESP" | "PRT" | "NLD" | "BEL" | "LUX"
            | "DNK" | "NOR" | "SWE" | "FIN" | "ISL" | "IRL" | "CHE" | "AUT" | "GRC" | "JAP" | "AST"
            | "NZL" | "KOR" | "ISR" => Self::Ocidental,
            _ => Self::TerceiroMundo,
        }
    }
}

/// Uma cidade a desenhar.
#[derive(Debug, Clone, PartialEq)]
pub struct CidadeEntrada {
    pub id: String,
    pub nome: String,
    pub pais_tag: String,
    /// (longitude, latitude) em graus.
    pub posicao: (f32, f32),
    /// Nível de cidade 1..=8 (fracionário permitido).
    pub nivel: f32,
    pub populacao: u32,
    pub eh_capital: bool,
    /// `None` = deduzido de `pais_tag`.
    pub bloco: Option<BlocoVisual>,
    /// `None` = era global de [`AnoVisual`]; `Some(0..=4)` = era local.
    pub era: Option<usize>,
    /// Nível máximo previsto (só dimensiona o fallback procedural). `None` = nível + 2.
    pub nivel_maximo: Option<u8>,
}

impl Default for CidadeEntrada {
    fn default() -> Self {
        Self {
            id: String::new(),
            nome: String::new(),
            pais_tag: String::new(),
            posicao: (0.0, 0.0),
            nivel: 1.0,
            populacao: 0,
            eh_capital: false,
            bloco: None,
            era: None,
            nivel_maximo: None,
        }
    }
}

/// Uma instalação industrial/extrativa a desenhar.
#[derive(Debug, Clone, PartialEq)]
pub struct InstalacaoEntrada {
    pub id: String,
    pub nome: String,
    pub tipo: String,
    pub pais_tag: String,
    /// (longitude, latitude) em graus.
    pub posicao: (f32, f32),
    /// Nível industrial (só informativo/HUD; o tamanho do sprite cresce com ele).
    pub nivel: u8,
    /// Produção relativa 0..=1 (comanda a fumaça).
    pub producao: f32,
    /// Poluição relativa 0..=1 (taxa de fumaça).
    pub poluicao: f32,
    /// `false` = ainda não construída (escondida).
    pub ativa: bool,
    pub era: Option<usize>,
}

impl Default for InstalacaoEntrada {
    fn default() -> Self {
        Self {
            id: String::new(),
            nome: String::new(),
            tipo: "fabrica".into(),
            pais_tag: String::new(),
            posicao: (0.0, 0.0),
            nivel: 1,
            producao: 1.0,
            poluicao: 0.5,
            ativa: true,
            era: None,
        }
    }
}

/// Conexão de transporte entre duas cidades (ids de [`CidadeEntrada`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConexaoEntrada {
    pub id: String,
    pub origem_id: String,
    pub destino_id: String,
    /// `estrada_terra`, `rodovia`, `ferrovia` ou `ferrovia_pesada`.
    pub tipo: String,
    /// Nível da via 1..=5 (espessura).
    pub nivel: u8,
    pub ativa: bool,
}

/// Tudo o que o plugin desenha. Escrito pelo jogo (ou pelo `DemoPlugin`).
#[derive(Resource, Debug, Clone, Default)]
pub struct EntradaVisual {
    pub cidades: Vec<CidadeEntrada>,
    pub instalacoes: Vec<InstalacaoEntrada>,
    pub conexoes: Vec<ConexaoEntrada>,
}

fn padrao_um_u32() -> u32 {
    1
}

fn padrao_um_f32() -> f32 {
    1.0
}

fn padrao_true() -> bool {
    true
}

/// Um prédio da simulação econômica para visualização no mapa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredioEconomiaEntrada {
    /// ID único do prédio ou identificador composto província/tipo.
    pub id: String,
    /// Tipo de prédio conforme `tipos_predio.ron` (ex.: "fazenda_graos", "siderurgica", "mina_carvao").
    pub tipo: String,
    /// ID do estado regional (ex.: "afg_cabul_central", "usa_new_york").
    pub estado_id: String,
    /// ID da província geográfica (ex.: "asm_4998", "BRA-1234"), se aplicável.
    #[serde(default)]
    pub provincia_id: Option<String>,
    /// Tag do país (ex.: "USA", "SOV", "BRA"). Se vazia, deduzida pelo estado.
    #[serde(default)]
    pub pais_tag: String,
    /// Nível de expansão da construção (1..N).
    #[serde(default = "padrao_um_u32")]
    pub nivel: u32,
    /// Métodos de produção ativos ou era tecnológica específica.
    #[serde(default)]
    pub metodos_ativos: Vec<String>,
    /// Taxa de produção/operação (0.0 a 1.0). Comanda a fumaça emitida.
    #[serde(default = "padrao_um_f32")]
    pub producao: f32,
    /// Posição geográfica explícita (lon, lat) se conhecida. Se `None`, posiciona via espiral no centroide.
    #[serde(default)]
    pub posicao: Option<(f32, f32)>,
    /// Bloco arquitetônico explícito. Se `None`, deduzido da tag do país.
    #[serde(default)]
    pub bloco: Option<BlocoVisual>,
    /// Era visual específica (0..=4). Se `None`, deduzida do relógio global.
    #[serde(default)]
    pub era: Option<usize>,
    /// Se o prédio está ativo/construído no momento.
    #[serde(default = "padrao_true")]
    pub ativa: bool,
}

impl Default for PredioEconomiaEntrada {
    fn default() -> Self {
        Self {
            id: String::new(),
            tipo: "fabrica_generica".into(),
            estado_id: String::new(),
            provincia_id: None,
            pais_tag: String::new(),
            nivel: 1,
            metodos_ativos: Vec::new(),
            producao: 1.0,
            posicao: None,
            bloco: None,
            era: None,
            ativa: true,
        }
    }
}

/// Dados de cidade urbana para visualização no mapa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CidadeDemograficaEntrada {
    pub id: String,
    pub nome: String,
    pub estado_id: String,
    pub pais_tag: String,
    pub populacao_urbana: u64,
    pub eh_capital: bool,
    /// (longitude, latitude) em graus.
    pub posicao: (f32, f32),
    #[serde(default)]
    pub bloco: Option<BlocoVisual>,
}

impl Default for CidadeDemograficaEntrada {
    fn default() -> Self {
        Self {
            id: String::new(),
            nome: String::new(),
            estado_id: String::new(),
            pais_tag: String::new(),
            populacao_urbana: 0,
            eh_capital: false,
            posicao: (0.0, 0.0),
            bloco: None,
        }
    }
}

/// Interface de entrada contendo a lista de construções e cidades da economia real.
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntradaEconomiaVisual {
    pub predios: Vec<PredioEconomiaEntrada>,
    pub cidades: Vec<CidadeDemograficaEntrada>,
}

impl EntradaEconomiaVisual {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_predios(&self) -> usize {
        self.predios.len()
    }

    pub fn total_cidades(&self) -> usize {
        self.cidades.len()
    }

    pub fn adicionar_predio(&mut self, p: PredioEconomiaEntrada) {
        self.predios.push(p);
    }

    pub fn adicionar_cidade(&mut self, c: CidadeDemograficaEntrada) {
        self.cidades.push(c);
    }
}

/// Trait para componentes ou simuladores que fornecem dados econômicos para o mapa.
pub trait FonteEconomiaVisual {
    fn predios_economia(&self) -> Vec<PredioEconomiaEntrada>;
    fn cidades_demograficas(&self) -> Vec<CidadeDemograficaEntrada>;
}

impl FonteEconomiaVisual for EntradaEconomiaVisual {
    fn predios_economia(&self) -> Vec<PredioEconomiaEntrada> {
        self.predios.clone()
    }

    fn cidades_demograficas(&self) -> Vec<CidadeDemograficaEntrada> {
        self.cidades.clone()
    }
}

/// Converte população urbana para nível de cidade de 1 a 8, conforme os parâmetros de `arte_cidades/cidades.ron`.
///
/// | Nível | Classificação | População |
/// | :--- | :--- | :--- |
/// | 1 | Aldeia / Povoado Rural | 0 - 20.000 |
/// | 2 | Vila / Pequena Cidade | 20.001 - 80.000 |
/// | 3 | Cidade Pequena | 80.001 - 250.000 |
/// | 4 | Cidade Média | 250.001 - 600.000 |
/// | 5 | Cidade Grande | 600.001 - 1.500.000 |
/// | 6 | Centro Regional | 1.500.001 - 4.000.000 |
/// | 7 | Metrópole Nacional | 4.000.001 - 10.000.000 |
/// | 8 | Megalópole Global | > 10.000.000 |
pub fn populacao_urbana_para_nivel(pop: u64) -> u8 {
    match pop {
        0..=20_000 => 1,
        20_001..=80_000 => 2,
        80_001..=250_000 => 3,
        250_001..=600_000 => 4,
        600_001..=1_500_000 => 5,
        1_500_001..=4_000_000 => 6,
        4_000_001..=10_000_000 => 7,
        _ => 8,
    }
}

/// Converte população urbana para nível fracionário suave no intervalo [1.0..=8.0].
pub fn populacao_urbana_para_nivel_fracionario(pop: u64) -> f32 {
    let niv = populacao_urbana_para_nivel(pop);
    let (min, max) = match niv {
        1 => (0.0, 20_000.0),
        2 => (20_000.0, 80_000.0),
        3 => (80_000.0, 250_000.0),
        4 => (250_000.0, 600_000.0),
        5 => (600_000.0, 1_500_000.0),
        6 => (1_500_000.0, 4_000_000.0),
        7 => (4_000_000.0, 10_000_000.0),
        _ => (10_000_000.0, 50_000_000.0),
    };
    let prog = ((pop as f32 - min) / (max - min)).clamp(0.0, 1.0);
    niv as f32 + prog * 0.99
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloco_por_pais() {
        assert_eq!(BlocoVisual::de_pais("USA"), BlocoVisual::Ocidental);
        assert_eq!(BlocoVisual::de_pais("SOV"), BlocoVisual::Sovietica);
        assert_eq!(BlocoVisual::de_pais("BRA"), BlocoVisual::TerceiroMundo);
        assert_eq!(BlocoVisual::TerceiroMundo.chave_atlas(), "terceiro_mundo");
    }

    #[test]
    fn niveis_populacao_urbana() {
        assert_eq!(populacao_urbana_para_nivel(15_000), 1);
        assert_eq!(populacao_urbana_para_nivel(50_000), 2);
        assert_eq!(populacao_urbana_para_nivel(150_000), 3);
        assert_eq!(populacao_urbana_para_nivel(450_000), 4);
        assert_eq!(populacao_urbana_para_nivel(1_000_000), 5);
        assert_eq!(populacao_urbana_para_nivel(2_500_000), 6);
        assert_eq!(populacao_urbana_para_nivel(7_000_000), 7);
        assert_eq!(populacao_urbana_para_nivel(12_000_000), 8);
    }

    #[test]
    fn trait_fonte_economia() {
        let mut e = EntradaEconomiaVisual::new();
        e.adicionar_predio(PredioEconomiaEntrada {
            id: "teste_1".into(),
            tipo: "fazenda_graos".into(),
            estado_id: "estado_a".into(),
            nivel: 3,
            ..default()
        });
        assert_eq!(e.total_predios(), 1);
        assert_eq!(e.predios_economia().len(), 1);
    }
}
