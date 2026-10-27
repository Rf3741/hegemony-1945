//! Paleta de cores e tokens de interface para Guerra Fria 1945.
//! Baseado nas especificações de `design/1_identidade` e `design/2_telas`.

use bevy_egui::egui::Color32;

/// Superfícies e fundos escuros do gabinete de guerra (Dark Slate / Baquelite).
pub mod superficies {
    use egui::Color32;
    pub const BG_VOID: Color32 = Color32::from_rgb(15, 17, 18);
    pub const BG_BASE: Color32 = Color32::from_rgb(24, 27, 29);
    pub const BG_SURFACE: Color32 = Color32::from_rgb(34, 38, 41);
    pub const BG_SURFACE_ELEVATED: Color32 = Color32::from_rgb(45, 50, 55);
    pub const BG_SURFACE_SUBTLE: Color32 = Color32::from_rgb(20, 22, 24);
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(46, 53, 59);
    pub const BORDER_MEDIUM: Color32 = Color32::from_rgb(65, 74, 82);
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(104, 117, 130);
}

/// Dossiê em papel envelhecido e telegramas de gabinete.
pub mod dossie_papel {
    use egui::Color32;
    pub const PAPEL_BASE: Color32 = Color32::from_rgb(217, 207, 184);
    pub const PAPEL_AMARELADO: Color32 = Color32::from_rgb(203, 191, 163);
    pub const PAPEL_CLARO: Color32 = Color32::from_rgb(235, 228, 211);
    pub const PAPEL_BORDA: Color32 = Color32::from_rgb(158, 145, 118);
    pub const TINTA_MAQUINA: Color32 = Color32::from_rgb(35, 32, 27);
    pub const TINTA_DESGASTADA: Color32 = Color32::from_rgb(89, 83, 71);
    pub const CARIMBO_VERMELHO: Color32 = Color32::from_rgb(163, 39, 34);
}

/// Tipografia e cores de texto legíveis.
pub mod texto {
    use egui::Color32;
    pub const PRIMARIO: Color32 = Color32::from_rgb(230, 236, 239);
    pub const SECUNDARIO: Color32 = Color32::from_rgb(155, 167, 176);
    pub const DESATIVADO: Color32 = Color32::from_rgb(104, 116, 126);
    pub const INVERTIDO: Color32 = Color32::from_rgb(18, 20, 22);
    pub const OURO_DESTAQUE: Color32 = Color32::from_rgb(223, 177, 91);
}

/// Cores semânticas de status (economia, estabilidade, alertas).
pub mod status {
    use egui::Color32;
    pub const SUCESSO: Color32 = Color32::from_rgb(76, 138, 77);
    pub const SUCESSO_FUNDO: Color32 = Color32::from_rgb(26, 43, 28);
    pub const PERIGO: Color32 = Color32::from_rgb(184, 50, 40);
    pub const PERIGO_FUNDO: Color32 = Color32::from_rgb(49, 22, 21);
    pub const AVISO: Color32 = Color32::from_rgb(200, 134, 39);
    pub const AVISO_FUNDO: Color32 = Color32::from_rgb(46, 33, 16);
    pub const INFO: Color32 = Color32::from_rgb(56, 115, 158);
    pub const INFO_FUNDO: Color32 = Color32::from_rgb(18, 34, 45);
}

/// Blocos geopolíticos da Guerra Fria.
pub mod blocos {
    use egui::Color32;
    pub const OCIDENTAL_PRIMARIA: Color32 = Color32::from_rgb(41, 92, 143);
    pub const OCIDENTAL_CLARA: Color32 = Color32::from_rgb(78, 133, 189);
    pub const OCIDENTAL_ESCURA: Color32 = Color32::from_rgb(24, 55, 86);
    pub const OCIDENTAL_REALCE: Color32 = Color32::from_rgb(106, 165, 222);

    pub const SOVIETICO_PRIMARIA: Color32 = Color32::from_rgb(170, 41, 36);
    pub const SOVIETICO_CLARA: Color32 = Color32::from_rgb(207, 66, 60);
    pub const SOVIETICO_ESCURA: Color32 = Color32::from_rgb(102, 24, 21);
    pub const SOVIETICO_OURO: Color32 = Color32::from_rgb(217, 167, 56);

    pub const NAO_ALINHADO_PRIMARIA: Color32 = Color32::from_rgb(108, 127, 95);
    pub const NAO_ALINHADO_CLARA: Color32 = Color32::from_rgb(142, 163, 127);
    pub const NAO_ALINHADO_ESCURA: Color32 = Color32::from_rgb(63, 76, 54);
}

/// Retorna a cor de destaque associada à tag do país.
pub fn cor_pais(tag: &str) -> Color32 {
    match tag {
        "USA" => Color32::from_rgb(43, 83, 128),
        "SOV" => Color32::from_rgb(158, 38, 33),
        "GBR" => Color32::from_rgb(150, 73, 83),
        "FRA" => Color32::from_rgb(61, 101, 122),
        "CHI" => Color32::from_rgb(176, 125, 59),
        "BRA" => Color32::from_rgb(59, 107, 85),
        "GER" => Color32::from_rgb(77, 77, 77),
        "JAP" => Color32::from_rgb(115, 59, 67),
        _ => Color32::from_rgb(110, 115, 104),
    }
}
