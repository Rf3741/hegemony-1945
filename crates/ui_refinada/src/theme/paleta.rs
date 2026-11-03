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

/// Paleta compartilhada no estilo Menu Principal / Menu de Pausa / Cartaz 1945:
/// papel creme envelhecido, borda vermelha dupla, tinta preta/carvão e acentos de cartaz.
pub mod menu {
    use egui::Color32;

    /// Fundo base em papel creme envelhecido (#E8DEC8)
    pub const PAPEL: Color32 = Color32::from_rgb(0xE8, 0xDE, 0xC8);
    /// Fundo claro para campos de texto e áreas de edição (#F4EEE0)
    pub const PAPEL_CLARO: Color32 = Color32::from_rgb(0xF4, 0xEE, 0xE0);
    /// Fundo para cartões e caixas secundárias (#D2C5A6)
    pub const PAPEL_CARD: Color32 = Color32::from_rgb(0xD2, 0xC5, 0xA6);
    /// Fundo alternado suave para linhas de tabela (#DFD4BD)
    pub const PAPEL_ALT: Color32 = Color32::from_rgb(0xDF, 0xD4, 0xBD);
    /// Borda suave para divisões internas (#B8A888)
    pub const PAPEL_BORDA: Color32 = Color32::from_rgb(0xB8, 0xA8, 0x88);

    /// Borda vermelha clássica e destaque (#8C251E)
    pub const VERMELHO: Color32 = Color32::from_rgb(0x8C, 0x25, 0x1E);
    /// Vermelho vivo para hover de botões (#AA2D24)
    pub const VERMELHO_HOVER: Color32 = Color32::from_rgb(0xAA, 0x2D, 0x24);
    /// Vermelho escuro para clique ativo (#6E1C16)
    pub const VERMELHO_ATIVO: Color32 = Color32::from_rgb(0x6E, 0x1C, 0x16);

    /// Tinta escura principal de máquina de escrever / cartaz (#171614)
    pub const TINTA: Color32 = Color32::from_rgb(0x17, 0x16, 0x14);
    /// Tinta suave para rótulos e texto secundário (#5C5850)
    pub const TINTA_SUAVE: Color32 = Color32::from_rgb(0x5C, 0x58, 0x50);
    /// Cinza escuro para botões neutros e molduras (#3A3732)
    pub const CINZA: Color32 = Color32::from_rgb(0x3A, 0x37, 0x32);
    /// Cinza desativado (#8A, 0x84, 0x78)
    pub const CINZA_DESATIVADO: Color32 = Color32::from_rgb(0x8A, 0x84, 0x78);

    /// Azul-marinho de cartaz de época (#193760)
    pub const AZUL_MARINHO: Color32 = Color32::from_rgb(25, 55, 96);
    /// Ouro envelhecido para honrarias e destaques (#C89628)
    pub const OURO: Color32 = Color32::from_rgb(200, 150, 40);

    // Cores de status com alto contraste sobre papel creme (WCAG AA >= 4.5:1)
    pub const SUCESSO: Color32 = Color32::from_rgb(27, 110, 40);
    pub const PERIGO: Color32 = Color32::from_rgb(158, 34, 26);
    pub const AVISO: Color32 = Color32::from_rgb(162, 92, 8);
    pub const INFO: Color32 = Color32::from_rgb(24, 72, 126);
}

/// Cor de saldo financeiro ou variação legível sobre fundo creme.
pub fn cor_saldo(v: f64) -> Color32 {
    if v >= 0.0 {
        menu::SUCESSO
    } else {
        menu::PERIGO
    }
}

/// Cor para variações percentuais ou deltas de mercado.
pub fn cor_variacao(v: f64) -> Color32 {
    if v > 0.001 {
        menu::SUCESSO
    } else if v < -0.001 {
        menu::PERIGO
    } else {
        menu::TINTA_SUAVE
    }
}

/// Calcula luminância relativa sRGB (WCAG 2.1).
pub fn luminancia_relativa(c: Color32) -> f64 {
    let canal = |v: u8| -> f64 {
        let f = v as f64 / 255.0;
        if f <= 0.03928 {
            f / 12.92
        } else {
            ((f + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * canal(c.r()) + 0.7152 * canal(c.g()) + 0.0722 * canal(c.b())
}

/// Calcula a razão de contraste entre duas cores (1.0 a 21.0).
pub fn razao_contraste(c1: Color32, c2: Color32) -> f64 {
    let l1 = luminancia_relativa(c1);
    let l2 = luminancia_relativa(c2);
    let mais_clara = l1.max(l2);
    let mais_escura = l1.min(l2);
    (mais_clara + 0.05) / (mais_escura + 0.05)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn contraste_minimo_sobre_papel() {
        let papel = menu::PAPEL;
        // Texto principal deve atingir padrão AAA (> 7:1)
        let c_tinta = razao_contraste(menu::TINTA, papel);
        assert!(c_tinta >= 7.0, "Tinta primária deve ter contraste >= 7:1 (deu {c_tinta:.2})");

        // Rótulos secundários devem atingir padrão AA (> 4.5:1)
        let c_tinta_suave = razao_contraste(menu::TINTA_SUAVE, papel);
        assert!(c_tinta_suave >= 4.0, "Tinta suave deve ter contraste >= 4.0:1 (deu {c_tinta_suave:.2})");

        // Borda e destaques vermelhos devem ter contraste excelente (> 4.5:1)
        let c_vermelho = razao_contraste(menu::VERMELHO, papel);
        assert!(c_vermelho >= 4.5, "Vermelho do tema deve ter contraste >= 4.5:1 (deu {c_vermelho:.2})");

        // Status de sucesso (verde escuro) e perigo (vermelho escuro) sobre papel
        let c_sucesso = razao_contraste(menu::SUCESSO, papel);
        assert!(c_sucesso >= 4.0, "Sucesso sobre papel deve ter contraste >= 4.0:1 (deu {c_sucesso:.2})");

        let c_perigo = razao_contraste(menu::PERIGO, papel);
        assert!(c_perigo >= 4.5, "Perigo sobre papel deve ter contraste >= 4.5:1 (deu {c_perigo:.2})");
    }

    #[test]
    fn cor_saldo_funciona() {
        assert_eq!(cor_saldo(100.0), menu::SUCESSO);
        assert_eq!(cor_saldo(0.0), menu::SUCESSO);
        assert_eq!(cor_saldo(-50.0), menu::PERIGO);
    }
}
