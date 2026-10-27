use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::f64::consts::PI;

/// Largura padrão do bitmap de províncias (`assets/map/provinces.png`).
pub const LARGURA_MAPA_PADRAO: u32 = 5632;
/// Altura padrão do bitmap de províncias (`assets/map/provinces.png`).
pub const ALTURA_MAPA_PADRAO: u32 = 2048;
/// Cor reservada para oceanos, mares e grandes lagos sem soberania (`id = 0`).
pub const COR_OCEANO: (u8, u8, u8) = (0, 0, 0);

/// Gera uma cor RGB única, determinística e bijetiva para um `id` de província (`1..=65535`).
///
/// - `R` codifica os 8 bits inferiores (`id & 0xFF`).
/// - `G` codifica os 8 bits superiores (`(id >> 8) & 0xFF`).
/// - `B` recebe uma dispersão visual determinística no intervalo `32..=254`,
///   garantindo distinção nítida do oceano `(0, 0, 0)` e boa separação cromática ao inspecionar o PNG.
pub const fn cor_unica_provincia(id: u32) -> (u8, u8, u8) {
    if id == 0 {
        return COR_OCEANO;
    }
    let r = (id & 0xFF) as u8;
    let g = ((id >> 8) & 0xFF) as u8;
    let b = (((id.wrapping_mul(157)) % 223) + 32) as u8;
    (r, g, b)
}

/// Decodifica e valida uma cor RGB do bitmap `provinces.png` de volta ao `id` da província (`None` para oceano ou cor inválida).
pub const fn id_de_cor_provincia(cor: (u8, u8, u8)) -> Option<u32> {
    if cor.0 == 0 && cor.1 == 0 && cor.2 == 0 {
        return None;
    }
    let id = (cor.0 as u32) | ((cor.1 as u32) << 8);
    if id == 0 {
        return None;
    }
    let esperada = cor_unica_provincia(id);
    if esperada.2 == cor.2 {
        Some(id)
    } else {
        None
    }
}

/// Constante máxima vertical da Projeção Cilíndrica de Miller para latitude `±90°`.
const MILLER_Y_MAX: f64 = 2.3034125433763912; // 1.25 * ln(tan(pi/4 + 0.4 * pi/2))

/// Projeta coordenadas geográficas `(lon_graus, lat_graus)` em coordenadas contínuas de pixel `(x, y)`
/// usando a **Projeção Cilíndrica de Miller** sobre `largura x altura`.
pub fn projetar_miller(lon_graus: f64, lat_graus: f64, largura: u32, altura: u32) -> (f64, f64) {
    let lon = lon_graus.clamp(-180.0, 180.0);
    let lat = lat_graus.clamp(-90.0, 90.0);
    let phi = lat.to_radians();
    let y_miller = 1.25 * ((PI * 0.25) + 0.4 * phi).tan().ln();

    let u = (lon + 180.0) / 360.0;
    let v = (MILLER_Y_MAX - y_miller) / (2.0 * MILLER_Y_MAX);

    (u * (largura as f64), v * (altura as f64))
}

/// Inversa exata da Projeção Cilíndrica de Miller: converte `(px, py)` de volta para `(lon_graus, lat_graus)`.
pub fn desprojetar_miller(px: f64, py: f64, largura: u32, altura: u32) -> (f64, f64) {
    let u = (px / (largura as f64)).clamp(0.0, 1.0);
    let v = (py / (altura as f64)).clamp(0.0, 1.0);

    let lon = u * 360.0 - 180.0;
    let y_miller = MILLER_Y_MAX - v * (2.0 * MILLER_Y_MAX);
    let phi = ((y_miller / 1.25).exp().atan() - (PI * 0.25)) / 0.4;
    let lat = phi.to_degrees().clamp(-90.0, 90.0);

    (lon, lat)
}

/// Registro cartográfico de uma província em `assets/map/provinces.ron`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvinciaMapa {
    /// Identificador numérico sequencial (`1..=N`).
    pub id: u32,
    /// Código estável do Natural Earth 10m Admin-1 (ex: `"BRA-1311"`, `"USA-3521"`).
    pub adm1_code: String,
    /// Nome de exibição em português brasileiro (ou topônimo local quando não traduzido).
    pub nome: String,
    /// Cor RGB única no bitmap `assets/map/provinces.png`.
    pub cor: (u8, u8, u8),
    /// Centroide geográfico `(longitude, latitude)` em graus decimais.
    pub centroide: (f32, f32),
    /// Posição em pixels `(x, y)` garantidamente sobre a província em `provinces.png`.
    pub centroide_px: (u32, u32),
    /// Área territorial aproximada em quilômetros quadrados (`km²`).
    pub area: f32,
    /// Lista ordenada de IDs de províncias vizinhas por fronteira terrestre/costeira imediata.
    pub vizinhos: Vec<u32>,
    /// Nome do país moderno correspondente na base do Natural Earth.
    pub pais_moderno: String,
    /// Código ISO/ADM0 de 3 letras do país moderno no Natural Earth (ex: `"RUS"`, `"LTU"`, `"VNM"`).
    pub adm0_moderno: String,
}

/// Catálogo completo serializado em `assets/map/provinces.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogoProvincias {
    pub largura: u32,
    pub altura: u32,
    pub projecao: String,
    pub provincias: Vec<ProvinciaMapa>,
}

impl CatalogoProvincias {
    /// Constrói tabela de busca rápida `cor RGB -> id da província`.
    pub fn mapa_cor_para_id(&self) -> HashMap<(u8, u8, u8), u32> {
        self.provincias.iter().map(|p| (p.cor, p.id)).collect()
    }

    /// Constrói tabela de busca rápida `id -> índice no vetor`.
    pub fn mapa_id_para_indice(&self) -> HashMap<u32, usize> {
        self.provincias
            .iter()
            .enumerate()
            .map(|(idx, p)| (p.id, idx))
            .collect()
    }

    pub fn obter_por_id(&self, id: u32) -> Option<&ProvinciaMapa> {
        if id == 0 {
            return None;
        }
        let idx = (id - 1) as usize;
        if let Some(p) = self.provincias.get(idx)
            && p.id == id
        {
            return Some(p);
        }
        self.provincias.iter().find(|p| p.id == id)
    }
}

/// Bloco geopolítico inicial em 1º de janeiro de 1945.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlinhamentoGeopolitico {
    BlocoOcidental,
    BlocoSovietico,
    NaoAlinhado,
    OcupacaoAliada,
}

impl AlinhamentoGeopolitico {
    pub const fn rotulo_pt(self) -> &'static str {
        match self {
            Self::BlocoOcidental => "Bloco Ocidental (Liderança EUA/Aliados)",
            Self::BlocoSovietico => "Bloco Soviético (Liderança URSS)",
            Self::NaoAlinhado => "Não Alinhado / Neutro",
            Self::OcupacaoAliada => "Sob Ocupação / Transição de Guerra",
        }
    }
}

/// Definição histórica de um país, colônia ou zona ocupada em 01/01/1945 (`data/scenario_1945/countries_1945.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinicaoPais1945 {
    /// Tag única de 3 letras (ex: `"USA"`, `"SOV"`, `"GBR"`, `"IND"`, `"FIC"`, `"BRA"`).
    pub tag: String,
    /// Nome histórico em português brasileiro em 1945.
    pub nome: String,
    /// Cor política RGB no mapa.
    pub cor: (u8, u8, u8),
    /// Cidade capital ou sede administrativa colonial.
    pub capital: String,
    /// Tag da metrópole/potência ocupante caso seja colônia, protetorado ou mandato.
    pub suserano: Option<String>,
    /// Alinhamento inicial na ordem pós-guerra.
    pub alinhamento: AlinhamentoGeopolitico,
    /// Ideologia governante em 1945.
    pub ideologia: String,
    /// População histórica aproximada em 1945 (caso 0, estimada proporcionalmente às províncias).
    pub populacao_1945: u64,
    /// PIB anual estimado em milhões de dólares internacionais de 1945/1990.
    pub pib_milhoes_usd: u64,
    /// Leis iniciais ativas nas 7 categorias institucionais.
    pub leis_iniciais: BTreeMap<String, String>,
}

/// Regras declarativas de ajuste territorial para 1945 (`data/scenario_1945/rules_1945.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegrasCenario1945 {
    /// Mapeia `adm0_moderno` do Natural Earth para a `tag` histórica de 1945.
    pub adm0_para_tag_1945: BTreeMap<String, String>,
    /// Sobrescritas finas por `adm1_code` (ex: Terra Nova -> `NEW`, Goa -> `POR`, Kaliningrado -> `SOV`).
    pub overrides_adm1: BTreeMap<String, String>,
}

/// Tabela resolvida de soberania/posse das províncias em 01/01/1945 (`data/scenario_1945/province_owners_1945.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DonosProvincias1945 {
    /// Mapa `id_provincia -> tag_pais_1945`.
    pub por_id: BTreeMap<u32, String>,
    /// Mapa `adm1_code -> tag_pais_1945` para auditoria legível e estabilidade entre regenerações.
    pub por_adm1: BTreeMap<String, String>,
}
