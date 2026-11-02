//! Motor principal da simulação política (`MundoPolitico`).
//!
//! Orquestra a execução do loop diário de 6 fases estruturais:
//! 1. Força política e Clout dos Grupos de Interesse;
//! 2. Dinâmica de Radicais e Leais dos Pops;
//! 3. Aprovação (-20..=+20) e Humor dos GIs;
//! 4. Legitimidade do gabinete de governo (0..=100);
//! 5. Tramitação legislativa e deliberação parlamentar;
//! 6. Esferas de influência da Guerra Fria, Eleições e Crises / Guerra Civil.

pub mod aprovacao;
pub mod eleicoes;
pub mod esferas_influencia;
pub mod forca_politica;
pub mod legislativo;
pub mod legitimidade;
pub mod movimentos_crises;
pub mod partidos;
pub mod radicalismo;

use crate::catalogo::CatalogoPolitica;
use crate::entidades::crise::SinalGuerraCivil;
use crate::entidades::grupo_interesse::InstanciaGrupoInteresse;
use crate::entidades::lider::LiderPolitico;
use crate::entidades::partido::InstanciaPartido;
use crate::entidades::pais::PaisPolitica;
use crate::entidades::pop::PopPolitica;
use crate::erro::{ErroPolitica, ResultadoPolitica, validar_id_snake_case};
use crate::rng::RngDeterministico;
use crate::tipos::{DataJogo, Estrato, PapelGrupoInteresse, StatusClout, HumorGrupoInteresse};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Resumo executivo retornado ao final de cada tick diário.
#[derive(Debug, Clone, Default)]
pub struct RelatorioTickPolitico {
    pub data: DataJogo,
    pub dias_decorridos: u64,
    pub populacao_mundial: f64,
    pub leis_promulgadas: usize,
    pub eleicoes_realizadas: usize,
    pub novas_guerras_civis: usize,
}

/// Instância do mundo político em execução com suporte a determinismo rigoroso via seed.
#[derive(Debug, Clone)]
pub struct MundoPolitico {
    data_atual: DataJogo,
    dias_decorridos: u64,
    catalogo: CatalogoPolitica,
    paises: Vec<PaisPolitica>,
    indice_paises: BTreeMap<String, usize>,
    rng: RngDeterministico,
    eventos_guerra_civil: Vec<SinalGuerraCivil>,
}

impl MundoPolitico {
    /// Insere uma nação surgida na descolonização com um estado político jogável.
    pub fn adicionar_pais_formado(
        &mut self,
        tag: &str,
        nome: &str,
        populacao: u64,
        leis: &BTreeMap<String, String>,
        ideologia: &str,
    ) -> ResultadoPolitica<()> {
        if self.paises.iter().any(|p| p.tag == tag) {
            return Ok(());
        }
        let id = tag.to_ascii_lowercase();
        let ideologia = if self.catalogo.ideologias.contains_key(ideologia) {
            ideologia.to_string()
        } else {
            self.catalogo.ideologias.keys().next().cloned().unwrap_or_else(|| "nacionalismo_republicano".into())
        };
        let mut pais = PaisPolitica::novo(&id, tag, nome)?;
        pais.leis_ativas = leis
            .iter()
            .filter_map(|(categoria, lei)| {
                let categoria = match categoria.as_str() {
                    "governo" => "forma_governo",
                    "sistema_tributario" => "tributacao",
                    "saude_e_educacao" => "saude",
                    "exercito" => "policia_seguranca",
                    outro => outro,
                };
                self.catalogo.categorias_leis.get(categoria)
                    .filter(|c| c.leis.iter().any(|l| l.id == *lei))
                    .map(|_| (categoria.to_string(), lei.clone()))
            })
            .collect();
        if pais.leis_ativas.is_empty() {
            pais.leis_ativas.insert("forma_governo".into(), "republica_presidencialista".into());
            pais.leis_ativas.insert("sistema_eleitoral".into(), "sufragio_universal".into());
        }
        pais.pops.push(PopPolitica::novo("camponeses", "local", "religiao_local", Estrato::Baixo, populacao as f64, 8.0, 0.25));
        pais.culturas_primarias.push("local".into());
        for (i, def) in self.catalogo.grupos_interesse.values().take(3).enumerate() {
            let papel = if i == 0 { PapelGrupoInteresse::NoGoverno } else { PapelGrupoInteresse::NaOposicao };
            let mut gi = InstanciaGrupoInteresse::novo(def.id.clone(), def.nome.clone(), papel);
            gi.clout = if i == 0 { 0.5 } else { 0.25 };
            gi.status = if i == 0 { StatusClout::Poderoso } else { StatusClout::Influente };
            gi.humor = HumorGrupoInteresse::Neutro;
            pais.grupos_interesse.insert(def.id.clone(), gi);
        }
        for (i, arq) in self.catalogo.arquetipos_partido.values().take(2).enumerate() {
            let mut partido = InstanciaPartido::novo(format!("{tag}_{}", arq.id.to_ascii_lowercase()), arq.nome.clone(), arq.id.clone());
            partido.grupos_membros = pais.grupos_interesse.keys().skip(i).take(1).cloned().collect();
            pais.partidos.insert(partido.id.clone(), partido);
        }
        let lider = LiderPolitico::novo(format!("lider_{id}"), nome, &ideologia, 55.0, 1.0, 1920);
        pais.chefe_de_estado = Some(lider.clone());
        pais.chefe_de_governo = Some(lider);
        pais.gi_chefe_estado = pais.grupos_interesse.keys().next().cloned();
        let idx = self.paises.len();
        self.indice_paises.insert(id, idx);
        self.paises.push(pais);
        Ok(())
    }

    /// Inicializa e valida um novo mundo político determinístico a partir de uma semente.
    pub fn novo(
        catalogo: CatalogoPolitica,
        mut paises: Vec<PaisPolitica>,
        semente: u64,
    ) -> ResultadoPolitica<Self> {
        catalogo.validar()?;

        let mut indice = BTreeMap::new();
        for (idx, p) in paises.iter_mut().enumerate() {
            validar_id_snake_case("PaisPolitica", &p.id)?;
            if indice.insert(p.id.clone(), idx).is_some() {
                return Err(ErroPolitica::IdentificadorDuplicado {
                    entidade: "PaisPolitica",
                    id: p.id.clone(),
                });
            }

            // Inicializa partidos dinâmicos
            partidos::atualizar_partidos_dinamicos(p, &catalogo);
            // Calcula força inicial e clout
            forca_politica::calcular_forca_politica_e_clout(p, &catalogo);
            // Calcula legitimidade inicial
            legitimidade::calcular_legitimidade_governo(p, &catalogo);
            // Calcula aprovação inicial
            aprovacao::atualizar_aprovacao_grupos_interesse(p, &catalogo);
        }

        Ok(Self {
            data_atual: DataJogo::nova(1945, 1, 1).unwrap(),
            dias_decorridos: 0,
            catalogo,
            paises,
            indice_paises: indice,
            rng: RngDeterministico::com_semente(semente),
            eventos_guerra_civil: Vec::new(),
        })
    }

    /// Retorna a data histórica atual do mundo político.
    #[inline]
    pub fn data_atual(&self) -> DataJogo {
        self.data_atual
    }

    /// Retorna a quantidade total de ticks diários transcorridos.
    #[inline]
    pub fn dias_decorridos(&self) -> u64 {
        self.dias_decorridos
    }

    /// Retorna a lista imutável de países simulados.
    #[inline]
    pub fn paises(&self) -> &[PaisPolitica] {
        &self.paises
    }

    /// Retorna a lista mutável de países simulados.
    #[inline]
    pub fn paises_mut(&mut self) -> &mut [PaisPolitica] {
        &mut self.paises
    }

    /// Busca um país pelo seu ID em snake_case ASCII.
    pub fn pais(&self, id: &str) -> Option<&PaisPolitica> {
        self.indice_paises.get(id).map(|&idx| &self.paises[idx])
    }

    /// Busca um país mutável pelo seu ID.
    pub fn pais_mut(&mut self, id: &str) -> Option<&mut PaisPolitica> {
        let &idx = self.indice_paises.get(id)?;
        Some(&mut self.paises[idx])
    }

    /// Retorna a referência ao catálogo estático de leis, ideologias e grupos.
    #[inline]
    pub fn catalogo(&self) -> &CatalogoPolitica {
        &self.catalogo
    }

    /// Retorna e esvazia a fila de eventos de Guerra Civil transmitidos para o `sim_guerra`.
    pub fn drenar_eventos_guerra_civil(&mut self) -> Vec<SinalGuerraCivil> {
        std::mem::take(&mut self.eventos_guerra_civil)
    }

    /// Executa exatamente um tick diário em todos os países do mundo político.
    pub fn executar_tick_diario(&mut self) -> RelatorioTickPolitico {
        let mut relatorio = RelatorioTickPolitico {
            data: self.data_atual,
            dias_decorridos: self.dias_decorridos,
            ..Default::default()
        };

        for p in &mut self.paises {
            let leis_antes = p.leis_ativas.clone();
            let eleicoes_antes = p.ultima_eleicao.as_ref().map(|e| e.data);

            // Fase 1: Esferas de influência externa e financiamento internacional
            esferas_influencia::processar_esferas_e_influencia_externa(p);

            // Fase 2: Força política dos Pops e agregação de Clout por Grupo de Interesse
            forca_politica::calcular_forca_politica_e_clout(p, &self.catalogo);

            // Fase 3: Radicalismo e lealdade das populações
            radicalismo::atualizar_radicalismo_e_lealdade_pops(p);

            // Fase 4: Aprovação e humor dos Grupos de Interesse
            aprovacao::atualizar_aprovacao_grupos_interesse(p, &self.catalogo);

            // Fase 5: Formação e atualização dinâmica de Partidos Políticos
            partidos::atualizar_partidos_dinamicos(p, &self.catalogo);

            // Fase 6: Ciclo Eleitoral, campanhas e realização de pleitos
            eleicoes::processar_ciclo_eleitoral_diario(
                p,
                self.data_atual,
                &self.catalogo,
                &mut self.rng,
            );

            // Fase 7: Recálculo da Legitimidade Governamental
            legitimidade::calcular_legitimidade_governo(p, &self.catalogo);

            // Fase 8: Tramitação legislativa diária e checkpoints
            legislativo::processar_tramitacao_legislativa_diaria(p, &self.catalogo, &mut self.rng);

            // Fase 9: Movimentos políticos, termômetro revolucionário e conspirações de golpe
            let novos_sinais = movimentos_crises::processar_movimentos_e_crises(
                p,
                self.data_atual,
                &self.catalogo,
                &mut self.rng,
            );
            relatorio.novas_guerras_civis += novos_sinais.len();
            self.eventos_guerra_civil.extend(novos_sinais);

            // Contabilização no relatório
            relatorio.populacao_mundial += p.populacao_total();
            relatorio.leis_promulgadas += p
                .leis_ativas
                .iter()
                .filter(|(cat, lei)| leis_antes.get(*cat) != Some(*lei))
                .count();
            if p.ultima_eleicao.as_ref().map(|e| e.data) != eleicoes_antes {
                relatorio.eleicoes_realizadas += 1;
            }
        }

        // Avança o calendário gregoriano
        self.data_atual.avancar_dia();
        self.dias_decorridos += 1;

        relatorio
    }

    /// Executa um período consecutivo de N ticks diários.
    pub fn executar_periodo(&mut self, dias: u32) -> RelatorioTickPolitico {
        let mut rel = RelatorioTickPolitico::default();
        for _ in 0..dias {
            let r = self.executar_tick_diario();
            rel.leis_promulgadas += r.leis_promulgadas;
            rel.eleicoes_realizadas += r.eleicoes_realizadas;
            rel.novas_guerras_civis += r.novas_guerras_civis;
            rel.populacao_mundial = r.populacao_mundial;
        }
        rel.data = self.data_atual;
        rel.dias_decorridos = self.dias_decorridos;
        rel
    }

    /// Audita as invariantes numéricas do mundo político, verificando ausência de NaNs/Infs
    /// e limites de domínio.
    pub fn auditar_estabilidade(&self) -> ResultadoPolitica<()> {
        for p in &self.paises {
            if !p.legitimidade.is_finite() || !(0.0..=100.0).contains(&p.legitimidade) {
                return Err(ErroPolitica::InvarianteViolada {
                    contexto: format!("Pais '{}'", p.id),
                    detalhe: format!("legitimidade inválida: {}", p.legitimidade),
                });
            }

            let mut soma_clout = 0.0;
            for gi in p.grupos_interesse.values() {
                if !gi.clout.is_finite() || gi.clout < 0.0 || gi.clout > 1.0001 {
                    return Err(ErroPolitica::InvarianteViolada {
                        contexto: format!("GI '{}' em Pais '{}'", gi.id, p.id),
                        detalhe: format!("clout inválido: {}", gi.clout),
                    });
                }
                if !gi.aprovacao.is_finite() || !(-20.0001..=20.0001).contains(&gi.aprovacao) {
                    return Err(ErroPolitica::InvarianteViolada {
                        contexto: format!("GI '{}' em Pais '{}'", gi.id, p.id),
                        detalhe: format!("aprovação fora da faixa [-20, +20]: {}", gi.aprovacao),
                    });
                }
                soma_clout += gi.clout;
            }

            if !p.grupos_interesse.is_empty() && (soma_clout - 1.0).abs() > 0.02 {
                return Err(ErroPolitica::InvarianteViolada {
                    contexto: format!("Pais '{}'", p.id),
                    detalhe: format!("soma do clout dos IGs ({:.4}) difere de 1.0", soma_clout),
                });
            }

            for pop in &p.pops {
                pop.validar_invariantes()?;
            }
        }
        Ok(())
    }
}

/// Estado dinâmico do mundo político para salvar/carregar (o catálogo vem dos arquivos do jogo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstadoPoliticoSalvo {
    pub data: DataJogo,
    pub dias_decorridos: u64,
    pub paises: Vec<PaisPolitica>,
    pub rng: RngDeterministico,
    pub eventos_guerra_civil: Vec<SinalGuerraCivil>,
}

impl MundoPolitico {
    pub fn salvar_estado(&self) -> EstadoPoliticoSalvo {
        EstadoPoliticoSalvo {
            data: self.data_atual,
            dias_decorridos: self.dias_decorridos,
            paises: self.paises.clone(),
            rng: self.rng,
            eventos_guerra_civil: self.eventos_guerra_civil.clone(),
        }
    }

    pub fn restaurar_estado(&mut self, e: EstadoPoliticoSalvo) {
        self.data_atual = e.data;
        self.dias_decorridos = e.dias_decorridos;
        self.indice_paises = e.paises.iter().enumerate().map(|(i, p)| (p.id.clone(), i)).collect();
        self.paises = e.paises;
        self.rng = e.rng;
        self.eventos_guerra_civil = e.eventos_guerra_civil;
    }
}
