//! Motor principal de guerra e simulação determinística (`MundoGuerra`).

pub mod combate;
pub mod frentes;
pub mod ocupacao;
pub mod paz;
pub mod pontuacao;
pub mod suprimento;
pub mod territorio;

use crate::catalogo::CatalogoGuerra;
use crate::cenario::CenarioGuerra;
use crate::entidades::{Divisao, Exercito, Frente, Guerra, PaisMilitar, RegiaoMapa};
use crate::erro::{ErroGuerra, ResultadoGuerra, validar_id_snake_case, validar_tag_pais};
use crate::grafo::GrafoCompilado;
use crate::rng::RngDeterministico;
use crate::tipos::DataJogo;
use combate::RelatorioBatalha;
use paz::{AvaliacaoPaz, PropostaPaz, TratadoPazAssinado};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use suprimento::BuffersSuprimento;
use territorio::CacheTerritorial;

/// Relatório sintético das ocorrências em um único tick diário da simulação.
#[derive(Debug, Clone, Default)]
pub struct RelatorioTickGuerra {
    pub data: DataJogo,
    pub guerras_ativas: usize,
    pub frentes_ativas: usize,
    pub batalhas_ocorridas: usize,
    pub regioes_conquistadas: usize,
    pub baixas_totais_dia: u32,
    pub tratados_assinados: usize,
    pub detalhes_batalhas: Vec<RelatorioBatalha>,
}

/// Estado dinâmico da guerra para salvar/carregar (cenário + RNG + frentes ativas + tratados).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstadoGuerraSalvo {
    pub cenario: CenarioGuerra,
    pub rng: RngDeterministico,
    pub frentes_ativas: Vec<Frente>,
    pub historico_tratados: Vec<TratadoPazAssinado>,
    pub proximo_frente_id: u32,
}

/// Estado global dinâmico da simulação militar do mundo.
#[derive(Debug)]
pub struct MundoGuerra {
    pub cenario: CenarioGuerra,
    pub grafo: GrafoCompilado,
    pub rng: RngDeterministico,
    pub mapa_regioes: HashMap<String, usize>,
    pub mapa_paises: HashMap<String, usize>,
    pub mapa_comandantes: HashMap<String, usize>,
    pub custos_terreno_por_regiao: Vec<f64>,
    pub buffers_suprimento: BuffersSuprimento,
    /// Cache de posse territorial; use `invalidar_cache_territorial` ao editar regiões à mão.
    pub territorio: CacheTerritorial,
    pub proximo_frente_id: u32,
    pub frentes_ativas: Vec<Frente>,
    pub historico_tratados: Vec<TratadoPazAssinado>,
}

impl MundoGuerra {
    /// Inicializa e compila um mundo de guerra a partir de um cenário completo.
    pub fn novo(mut cenario: CenarioGuerra) -> ResultadoGuerra<Self> {
        cenario.catalogo.validar_e_indexar()?;

        let mut mapa_regioes = HashMap::with_capacity(cenario.regioes.len());
        for (i, r) in cenario.regioes.iter().enumerate() {
            if mapa_regioes.insert(r.adm1_code.clone(), i).is_some() {
                return Err(ErroGuerra::IdentificadorDuplicado {
                    entidade: "regiao",
                    id: r.adm1_code.clone(),
                });
            }
        }

        let mut mapa_paises = HashMap::with_capacity(cenario.paises.len());
        for (i, p) in cenario.paises.iter().enumerate() {
            validar_tag_pais(&p.tag)?;
            if mapa_paises.insert(p.tag.clone(), i).is_some() {
                return Err(ErroGuerra::IdentificadorDuplicado {
                    entidade: "pais",
                    id: p.tag.clone(),
                });
            }
        }

        let mut mapa_comandantes = HashMap::with_capacity(cenario.comandantes.len());
        for (i, c) in cenario.comandantes.iter().enumerate() {
            validar_id_snake_case("comandante", &c.id)?;
            if mapa_comandantes.insert(c.id.clone(), i).is_some() {
                return Err(ErroGuerra::IdentificadorDuplicado {
                    entidade: "comandante",
                    id: c.id.clone(),
                });
            }
        }

        let grafo =
            GrafoCompilado::construir(cenario.regioes.len(), &cenario.arestas, &mapa_regioes)?;

        let mut custos_terreno_por_regiao = Vec::with_capacity(cenario.regioes.len());
        for reg in &cenario.regioes {
            let t_idx = cenario
                .catalogo
                .mapa_terrenos
                .get(&reg.terreno_id)
                .copied()
                .unwrap_or(0);
            custos_terreno_por_regiao.push(cenario.catalogo.terrenos[t_idx].custo_suprimento);
        }

        let rng = RngDeterministico::novo(cenario.seed);
        let proximo_frente_id = 1;
        let buffers_suprimento = BuffersSuprimento::default();

        Ok(Self {
            cenario,
            grafo,
            rng,
            mapa_regioes,
            mapa_paises,
            mapa_comandantes,
            custos_terreno_por_regiao,
            buffers_suprimento,
            territorio: CacheTerritorial::default(),
            proximo_frente_id,
            frentes_ativas: Vec::new(),
            historico_tratados: Vec::new(),
        })
    }

    /// Executa o tick diário completo da simulação militar de forma determinística.
    pub fn executar_tick_diario(&mut self) -> RelatorioTickGuerra {
        // 1. Avança o calendário
        self.cenario.data_inicial.avancar_dia();
        let data = self.cenario.data_inicial;

        // Território só muda por conquistas/tratados: o cache sobrevive entre dias de paz
        self.territorio
            .garantir(&self.cenario.regioes, &self.cenario.paises);

        // 2. Calcula suprimento logístico das fontes para todas as divisões
        suprimento::calcular_suprimentos_mundo(
            &self.cenario.regioes,
            &self.grafo,
            &self.cenario.paises,
            &mut self.cenario.divisoes,
            &self.mapa_regioes,
            &self.mapa_paises,
            &self.custos_terreno_por_regiao,
            &self.territorio,
            &mut self.buffers_suprimento,
        );

        // 3. Atualiza frentes operacionais ativas na fronteira entre beligerantes
        self.frentes_ativas.clear();
        frentes::atualizar_frentes_mundo(
            &self.cenario.regioes,
            &self.grafo,
            &self.cenario.guerras,
            &mut self.frentes_ativas,
            &mut self.cenario.exercitos,
            &mut self.cenario.divisoes,
            &self.mapa_regioes,
            &mut self.proximo_frente_id,
        );

        // 4. Resolve combates diários ao longo das frentes e avança territórios
        let detalhes_batalhas = combate::resolver_combates_mundo(
            &mut self.cenario.regioes,
            &self.grafo,
            &self.cenario.catalogo,
            &mut self.cenario.paises,
            &self.cenario.comandantes,
            &mut self.cenario.divisoes,
            &self.cenario.exercitos,
            &self.frentes_ativas,
            &mut self.cenario.guerras,
            &self.cenario.config,
            &mut self.rng,
            &self.mapa_regioes,
            &self.mapa_paises,
            &self.mapa_comandantes,
        );

        if detalhes_batalhas.iter().any(|b| b.regiao_conquistada) {
            self.territorio.invalidar();
        }
        self.territorio
            .garantir(&self.cenario.regioes, &self.cenario.paises);

        // 5. Recuperação de organização, moral e acúmulo de entrincheiramento fora de combate
        for div in self.cenario.divisoes.iter_mut() {
            if div.efetivos_atuais == 0 {
                continue;
            }
            let tmpl_idx = self
                .cenario
                .catalogo
                .mapa_templates
                .get(&div.template_id)
                .copied()
                .unwrap_or(0);
            let st = &self.cenario.catalogo.templates_compilados[tmpl_idx];

            let sup_mult = (0.20 + 0.80 * div.suprimento_atual).clamp(0.20, 1.0);
            let delta_org =
                self.cenario.config.taxa_recuperacao_org_base * st.moral_recuperacao * sup_mult;
            div.organizacao = (div.organizacao + delta_org).min(st.organizacao_max);

            let delta_ent = self.cenario.config.taxa_acumulo_entrincheiramento * sup_mult;
            div.entrincheiramento =
                (div.entrincheiramento + delta_ent).min(st.entrincheiramento_max);
        }

        // 6. Atualiza ocupação territorial, resistência e condições de capitulação
        let status_territorial = ocupacao::atualizar_ocupacao_e_capitulacoes(
            &mut self.cenario.regioes,
            &self.cenario.paises,
            &mut self.cenario.guerras,
            &self.cenario.config,
            &self.mapa_regioes,
            &self.territorio,
        );

        // 7. Atualiza pontuação de guerra e cansaço de guerra
        pontuacao::atualizar_pontuacao_e_cansaco_mundo(
            &mut self.cenario.guerras,
            &mut self.cenario.paises,
            &status_territorial,
        );

        // 8. Avalia e aplica tratados de paz automáticos
        let novos_tratados = paz::verificar_fim_guerras_automatico(
            &mut self.cenario.guerras,
            &mut self.cenario.paises,
            &mut self.cenario.regioes,
            &mut self.frentes_ativas,
            &self.cenario.config,
            data,
            &self.mapa_regioes,
        );
        let n_tratados = novos_tratados.len();
        if n_tratados > 0 {
            self.territorio.invalidar();
        }
        self.historico_tratados.extend(novos_tratados);

        let mut baixas_dia = 0u32;
        let mut regioes_conquistadas = 0usize;
        for b in &detalhes_batalhas {
            baixas_dia += b.baixas_atacante + b.baixas_defensor;
            if b.regiao_conquistada {
                regioes_conquistadas += 1;
            }
        }

        let guerras_ativas = self.cenario.guerras.iter().filter(|g| g.ativa).count();
        let frentes_ativas = self.frentes_ativas.iter().filter(|f| f.ativa).count();

        RelatorioTickGuerra {
            data,
            guerras_ativas,
            frentes_ativas,
            batalhas_ocorridas: detalhes_batalhas.len(),
            regioes_conquistadas,
            baixas_totais_dia: baixas_dia,
            tratados_assinados: n_tratados,
            detalhes_batalhas,
        }
    }

    /// Propõe e tenta aplicar formalmente um tratado de paz.
    pub fn propor_paz(&mut self, proposta: PropostaPaz) -> ResultadoGuerra<TratadoPazAssinado> {
        let g_idx = self
            .cenario
            .guerras
            .iter()
            .position(|g| g.id == proposta.guerra_id)
            .ok_or_else(|| ErroGuerra::ReferenciaInexistente {
                origem_entidade: "tratado_paz",
                origem_id: "proposta".to_string(),
                alvo_entidade: "guerra",
                alvo_id: proposta.guerra_id.clone(),
            })?;

        let mut frentes_temp = Vec::new();
        let tratado = paz::aplicar_tratado_paz(
            g_idx,
            &mut self.cenario.guerras,
            &mut self.cenario.paises,
            &mut self.cenario.regioes,
            &mut frentes_temp,
            proposta,
            self.cenario.data_inicial,
            &self.mapa_regioes,
        )?;

        self.territorio.invalidar();
        self.historico_tratados.push(tratado.clone());
        Ok(tratado)
    }

    /// Avalia diplomaticamente se uma proposta de paz seria aceita pelo oponente.
    pub fn avaliar_paz(&self, proposta: &PropostaPaz) -> Option<AvaliacaoPaz> {
        let guerra = self
            .cenario
            .guerras
            .iter()
            .find(|g| g.id == proposta.guerra_id)?;
        Some(paz::avaliar_aceitacao_paz(
            guerra,
            &self.cenario.paises,
            proposta,
            &self.cenario.regioes,
            &self.mapa_regioes,
        ))
    }

    /// Declara uma nova guerra entre dois beligerantes.
    pub fn declarar_guerra(
        &mut self,
        atacante: &str,
        defensor: &str,
        nome_pt: &str,
    ) -> ResultadoGuerra<String> {
        validar_tag_pais(atacante)?;
        validar_tag_pais(defensor)?;

        if !self.mapa_paises.contains_key(atacante) {
            return Err(ErroGuerra::ReferenciaInexistente {
                origem_entidade: "guerra",
                origem_id: "nova".to_string(),
                alvo_entidade: "pais",
                alvo_id: atacante.to_string(),
            });
        }
        if !self.mapa_paises.contains_key(defensor) {
            return Err(ErroGuerra::ReferenciaInexistente {
                origem_entidade: "guerra",
                origem_id: "nova".to_string(),
                alvo_entidade: "pais",
                alvo_id: defensor.to_string(),
            });
        }

        let guerra_id = format!(
            "guerra_{}_{}_{}",
            atacante.to_lowercase(),
            defensor.to_lowercase(),
            self.cenario.data_inicial.ano
        );

        self.cenario.guerras.push(Guerra {
            id: guerra_id.clone(),
            nome_pt: nome_pt.to_string(),
            atacantes: vec![atacante.to_string()],
            defensores: vec![defensor.to_string()],
            data_inicio: self.cenario.data_inicial,
            pontuacao_guerra: 0.0,
            baixas_atacantes: 0,
            baixas_defensores: 0,
            batalhas_vencidas_atacantes: 0,
            batalhas_vencidas_defensores: 0,
            capitulou_atacantes: false,
            capitulou_defensores: false,
            ativa: true,
            envolvimentos: Vec::new(),
        });

        Ok(guerra_id)
    }

    /// Descarta o cache de posse territorial (chame após alterar donos/controladores à mão).
    pub fn invalidar_cache_territorial(&mut self) {
        self.territorio.invalidar();
    }

    // Acessores de conveniência
    pub fn regioes(&self) -> &[RegiaoMapa] {
        &self.cenario.regioes
    }

    pub fn paises(&self) -> &[PaisMilitar] {
        &self.cenario.paises
    }

    pub fn guerras(&self) -> &[Guerra] {
        &self.cenario.guerras
    }

    pub fn divisoes(&self) -> &[Divisao] {
        &self.cenario.divisoes
    }

    pub fn exercitos(&self) -> &[Exercito] {
        &self.cenario.exercitos
    }

    pub fn frentes(&self) -> &[Frente] {
        &self.frentes_ativas
    }

    pub fn exercitos_mut(&mut self) -> &mut [Exercito] {
        &mut self.cenario.exercitos
    }

    pub fn catalogo(&self) -> &CatalogoGuerra {
        &self.cenario.catalogo
    }

    /// Captura o estado dinâmico da guerra para salvar (cenário + RNG + frentes + tratados).
    pub fn salvar_estado(&self) -> EstadoGuerraSalvo {
        EstadoGuerraSalvo {
            cenario: self.cenario.clone(),
            rng: self.rng,
            frentes_ativas: self.frentes_ativas.clone(),
            historico_tratados: self.historico_tratados.clone(),
            proximo_frente_id: self.proximo_frente_id,
        }
    }

    /// Reconstrói o mundo de guerra a partir do estado salvo.
    pub fn restaurar_estado(_catalogo: CatalogoGuerra, e: EstadoGuerraSalvo) -> ResultadoGuerra<Self> {
        let mut m = Self::novo(e.cenario)?;
        m.rng = e.rng;
        m.frentes_ativas = e.frentes_ativas;
        m.historico_tratados = e.historico_tratados;
        m.proximo_frente_id = e.proximo_frente_id;
        m.invalidar_cache_territorial();
        Ok(m)
    }
}
