//! Gestão de fechamento temporário, reabertura com retomada gradual e demolição de construções.

use crate::catalogo::{CatalogoEconomia, TipoPredio};
use crate::entidades::{Estado, Mercado, Pais, Predio, Provincia};
use crate::erro::{ErroEconomia, ResultadoEconomia};
use crate::motor::MundoEconomico;
use crate::motor::distribuicao::RESERVA_CAIXA_POR_NIVEL;
use crate::motor::governo::{atualizar_terra_aravel_ocupada, materiais_por_ponto};
use crate::motor::trabalho::reconciliar_empregados_provincia;
use crate::tipos::CategoriaPredio;

/// Fração do valor em materiais de construção cobrada para reabrir e remobilizar um prédio fechado.
pub const FRACAO_CUSTO_REABRIR: f64 = 0.12;
/// Fração do valor em materiais de construção recuperada ao demolir níveis de um prédio.
pub const FRACAO_REEMBOLSO_DEMOLIR: f64 = 0.15;
/// Piso mínimo por nível do custo de reabertura.
pub const CUSTO_MINIMO_REABRIR_POR_NIVEL: f64 = 50.0;
/// Piso mínimo por nível do reembolso de demolição.
pub const REEMBOLSO_MINIMO_DEMOLIR_POR_NIVEL: f64 = 40.0;
/// Intervalo mínimo (em dias) entre mudanças automáticas de estado de um mesmo prédio pela IA/setor privado.
pub const HISTERESE_MUDANCA_PREDIO_DIAS: u64 = 120;
/// Dias acumulados de prejuízo exigidos para a IA ou setor privado fechar um prédio deficitário.
pub const DIAS_PREJUIZO_FECHAR_IA: u16 = 75;
/// Dias mínimos fechado antes de a IA ou setor privado considerar reabrir um prédio.
pub const DIAS_FECHADO_REABRIR_IA: u16 = 60;
/// Dias mínimos fechado no prejuízo estrutural antes de a IA ou setor privado demolir níveis.
pub const DIAS_FECHADO_DEMOLIR_IA: u16 = 270;

/// Resumo retornado pelas ações de fechar, reabrir ou demolir um prédio.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultadoAcaoPredio {
    pub pais_id: String,
    pub provincia_id: String,
    pub provincia_nome: String,
    pub tipo_predio_id: String,
    pub tipo_predio_nome: String,
    /// Quantidade de trabalhadores desligados imediatamente pela ação.
    pub trabalhadores_demitidos: f64,
    /// Fração da população adulta total da província atingida pela demissão (`0.0..=1.0`).
    pub fracao_forca_trabalho_provincia: f64,
    /// Valor monetário cobrado (reabertura) ou devolvido (demolição).
    pub valor_monetario: f64,
    /// Nível restante do prédio na província após a operação (`0` se demolido por completo).
    pub nivel_restante: u32,
}

/// Calcula o valor bruto em materiais de construção de `niveis` níveis usando uma função de preço por bem.
pub fn valor_materiais_construcao_niveis(
    _catalogo: &CatalogoEconomia,
    tipo_predio: &TipoPredio,
    niveis: u32,
    preco_bem: impl Fn(&str) -> f64,
) -> f64 {
    if niveis == 0 {
        return 0.0;
    }
    let mut por_nivel = 0.0;
    for &(bem_id, qtd_ponto) in materiais_por_ponto(tipo_predio.categoria) {
        por_nivel += qtd_ponto * tipo_predio.custo_pontos_construcao * preco_bem(bem_id).max(0.0);
    }
    por_nivel * (niveis as f64)
}

/// Calcula o custo monetário de reabertura para um prédio de `nivel` níveis.
pub fn calcular_custo_reabertura_por_precos(
    catalogo: &CatalogoEconomia,
    tipo_predio: &TipoPredio,
    nivel: u32,
    preco_bem: impl Fn(&str) -> f64,
) -> f64 {
    if nivel == 0 {
        return 0.0;
    }
    let base = valor_materiais_construcao_niveis(catalogo, tipo_predio, nivel, preco_bem);
    (base * FRACAO_CUSTO_REABRIR).max(CUSTO_MINIMO_REABRIR_POR_NIVEL * (nivel as f64))
}

/// Calcula o reembolso parcial de materiais recuperados ao demolir `niveis` níveis de um prédio.
pub fn calcular_reembolso_demolicao_por_precos(
    catalogo: &CatalogoEconomia,
    tipo_predio: &TipoPredio,
    niveis: u32,
    preco_bem: impl Fn(&str) -> f64,
) -> f64 {
    if niveis == 0 {
        return 0.0;
    }
    let base = valor_materiais_construcao_niveis(catalogo, tipo_predio, niveis, preco_bem);
    (base * FRACAO_REEMBOLSO_DEMOLIR).max(REEMBOLSO_MINIMO_DEMOLIR_POR_NIVEL * (niveis as f64))
}

/// Estima a margem operacional unitária (`(receita - insumos - folha) / folha`) de um prédio aos preços atuais,
/// mesmo que ele esteja atualmente fechado (`escala_efetiva == 0`).
pub fn estimar_margem_operacional_predio(
    catalogo: &CatalogoEconomia,
    mercado: &Mercado,
    estado: &Estado,
    pais: &Pais,
    predio: &Predio,
) -> f64 {
    if predio.nivel == 0 || predio.cache.financiado_pelo_governo {
        return 0.0;
    }
    let def = &catalogo.tipos_predio[predio.cache.tipo_idx];
    let econ_escala = 1.0 + 0.01 * (predio.nivel.saturating_sub(1).min(25) as f64);
    let acesso = estado.acesso_mercado.clamp(0.05, 1.0);
    let mult_qualidade = if predio.cache.qualidade_local.is_nan() {
        estado.rendimento_recurso(def)
    } else {
        predio.cache.qualidade_local
    };
    let mult_fabrica = if def.categoria == crate::tipos::CategoriaPredio::Industrial {
        let bonus_mercado = if acesso >= 0.90 { 0.03 } else { 0.0 };
        1.0 + bonus_mercado + estado.bonus_cadeia(predio.cache.tipo_idx)
    } else {
        1.0
    };

    let mut fator_insumos = 1.0_f64;
    for &(g_in, _) in predio.cache.insumos() {
        fator_insumos = fator_insumos.min(mercado.fator_suprimento_idx[g_in as usize]);
    }
    if fator_insumos < 0.25 {
        return -1.0;
    }

    let escala_por_nivel = econ_escala * acesso * fator_insumos * mult_qualidade * mult_fabrica;
    let precos = &mercado.precos_idx;

    let mut receita_por_nivel = 0.0;
    for &(g_out, qtd) in predio.cache.produtos() {
        receita_por_nivel += escala_por_nivel * pais.produtividade * qtd * precos[g_out as usize];
    }
    let mut custo_ins_por_nivel = 0.0;
    for &(g_in, qtd) in predio.cache.insumos() {
        custo_ins_por_nivel += escala_por_nivel * qtd * precos[g_in as usize];
    }
    let mut folha_por_nivel = 0.0;
    for slot in predio.cache.vagas() {
        let mult = catalogo.tipos_pop[slot.tipo_pop_idx as usize].multiplicador_salario;
        folha_por_nivel += (slot.vagas_por_nivel as f64) * predio.salario_base * mult;
    }

    if folha_por_nivel <= 1e-9 {
        0.0
    } else {
        (receita_por_nivel - custo_ins_por_nivel - folha_por_nivel) / folha_por_nivel
    }
}

fn sincronizar_mapa_empregados(catalogo: &CatalogoEconomia, predio: &mut Predio) {
    predio.empregados_por_tipo.clear();
    for slot in predio.cache.vagas() {
        if slot.empregados > 0.0
            && let Some(tp) = catalogo.tipos_pop.get(slot.tipo_pop_idx as usize)
        {
            predio.empregados_por_tipo.insert(tp.id.clone(), slot.empregados);
        }
    }
}

fn recalcular_vagas_abertas_provincia(prov: &mut Provincia) {
    prov.vagas_abertas_cache = prov
        .predios
        .iter()
        .filter(|b| b.nivel > 0 && !b.fechado)
        .map(|b| (b.total_vagas() * b.fator_retomada() - b.total_empregados()).max(0.0))
        .sum();
}

/// Atualiza em O(províncias do estado) os agregados locais do `estado` após fechar, reabrir ou demolir
/// um prédio em uma de suas províncias, sem varrer o mundo inteiro.
fn atualizar_estado_local(
    catalogo: &CatalogoEconomia,
    estado: &mut Estado,
    provincias: &[Provincia],
) {
    let nt = catalogo.tipos_predio.len();
    if estado.niveis_por_tipo.len() != nt {
        estado.niveis_por_tipo.resize(nt, 0);
    }
    estado.niveis_por_tipo.fill(0);
    estado.terra_aravel_ocupada = 0;

    let mut uso_infra = 0.0;
    for &pi in &estado.provincias_indices {
        let prov = &provincias[pi];
        estado.terra_aravel_ocupada = estado
            .terra_aravel_ocupada
            .saturating_add(prov.terra_aravel_ocupada);
        for b in &prov.predios {
            if b.nivel == 0 {
                continue;
            }
            if b.cache.tipo_idx < nt {
                estado.niveis_por_tipo[b.cache.tipo_idx] += b.nivel;
            }
            if b.fechado {
                continue;
            }
            let vagas = b.total_vagas();
            let r_emp = if vagas <= 0.0 {
                1.0
            } else {
                (b.total_empregados() / vagas).clamp(0.0, 1.0)
            };
            if r_emp <= 1e-7 {
                continue;
            }
            let escala_inst = (b.nivel as f64) * r_emp;
            let peso_uso = match catalogo.tipos_predio[b.cache.tipo_idx].categoria {
                CategoriaPredio::Industrial | CategoriaPredio::Extrativo => 2.0,
                CategoriaPredio::RuralAgricola
                | CategoriaPredio::Governamental
                | CategoriaPredio::Militar
                | CategoriaPredio::Construcao => 1.0,
                CategoriaPredio::Infraestrutura => 0.0,
            };
            uso_infra += escala_inst * peso_uso;
        }
    }
    estado.infraestrutura_uso = uso_infra;
    if estado.infraestrutura_uso <= estado.infraestrutura_capacidade
        || estado.infraestrutura_uso <= 1.0
    {
        estado.acesso_mercado = 1.0;
        estado.penalidade_infraestrutura = 0.0;
    } else {
        let cap = estado.infraestrutura_capacidade.max(0.0);
        let acesso = (cap / estado.infraestrutura_uso).clamp(0.05, 1.0);
        estado.acesso_mercado = acesso;
        estado.penalidade_infraestrutura = 1.0 - acesso;
    }
}

impl MundoEconomico {
    fn resolver_alvo_predio(
        &self,
        acao: &'static str,
        pais_id: &str,
        provincia_id: &str,
        tipo_predio_id: &str,
    ) -> ResultadoEconomia<(usize, usize, usize)> {
        let &c_idx = self.indice_paises.get(pais_id).ok_or_else(|| {
            ErroEconomia::ReferenciaInexistente {
                origem_entidade: acao,
                origem_id: pais_id.to_string(),
                alvo_entidade: "Pais",
                alvo_id: pais_id.to_string(),
            }
        })?;
        let &p_idx = self.indice_provincias.get(provincia_id).ok_or_else(|| {
            ErroEconomia::ReferenciaInexistente {
                origem_entidade: acao,
                origem_id: pais_id.to_string(),
                alvo_entidade: "Provincia",
                alvo_id: provincia_id.to_string(),
            }
        })?;
        let &tp_idx = self
            .catalogo
            .indice_tipos_predio
            .get(tipo_predio_id)
            .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                origem_entidade: acao,
                origem_id: pais_id.to_string(),
                alvo_entidade: "TipoPredio",
                alvo_id: tipo_predio_id.to_string(),
            })?;
        if self.provincias[p_idx].pais_idx != c_idx {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "Provincia",
                id: provincia_id.to_string(),
                detalhe: format!("a província não pertence ao país '{pais_id}'"),
            });
        }
        Ok((c_idx, p_idx, tp_idx))
    }

    /// Consulta o custo monetário estimado para reabrir um prédio fechado na província.
    pub fn custo_reabrir_predio(
        &self,
        pais_id: &str,
        provincia_id: &str,
        tipo_predio_id: &str,
    ) -> ResultadoEconomia<f64> {
        let (c_idx, p_idx, tp_idx) =
            self.resolver_alvo_predio("custo_reabrir_predio", pais_id, provincia_id, tipo_predio_id)?;
        let prov = &self.provincias[p_idx];
        let predio = prov
            .predios
            .iter()
            .find(|b| b.cache.tipo_idx == tp_idx && b.nivel > 0)
            .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                origem_entidade: "Provincia",
                origem_id: provincia_id.to_string(),
                alvo_entidade: "Predio",
                alvo_id: tipo_predio_id.to_string(),
            })?;
        let mercado = &self.mercados[self.paises[c_idx].mercado_idx];
        let def = &self.catalogo.tipos_predio[tp_idx];
        Ok(calcular_custo_reabertura_por_precos(
            &self.catalogo,
            def,
            predio.nivel,
            |bem_id| {
                self.catalogo
                    .indice_bens
                    .get(bem_id)
                    .map_or(20.0, |&g| mercado.precos_idx[g])
            },
        ))
    }

    /// Consulta o valor de sucata/materiais devolvido ao demolir `niveis_remover` níveis de um prédio.
    pub fn reembolso_demolir_predio(
        &self,
        pais_id: &str,
        provincia_id: &str,
        tipo_predio_id: &str,
        niveis_remover: u32,
    ) -> ResultadoEconomia<f64> {
        let (c_idx, p_idx, tp_idx) = self.resolver_alvo_predio(
            "reembolso_demolir_predio",
            pais_id,
            provincia_id,
            tipo_predio_id,
        )?;
        let prov = &self.provincias[p_idx];
        let predio = prov
            .predios
            .iter()
            .find(|b| b.cache.tipo_idx == tp_idx && b.nivel > 0)
            .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                origem_entidade: "Provincia",
                origem_id: provincia_id.to_string(),
                alvo_entidade: "Predio",
                alvo_id: tipo_predio_id.to_string(),
            })?;
        let removidos = niveis_remover.max(1).min(predio.nivel);
        let mercado = &self.mercados[self.paises[c_idx].mercado_idx];
        let def = &self.catalogo.tipos_predio[tp_idx];
        Ok(calcular_reembolso_demolicao_por_precos(
            &self.catalogo,
            def,
            removidos,
            |bem_id| {
                self.catalogo
                    .indice_bens
                    .get(bem_id)
                    .map_or(20.0, |&g| mercado.precos_idx[g])
            },
        ))
    }

    /// Fecha temporariamente um prédio: cessa produção e consumo de insumos, demite todos os
    /// trabalhadores (que voltam imediatamente a procurar emprego) e passa a cobrar manutenção baixa.
    pub fn fechar_predio(
        &mut self,
        pais_id: &str,
        provincia_id: &str,
        tipo_predio_id: &str,
    ) -> ResultadoEconomia<ResultadoAcaoPredio> {
        let (_c_idx, p_idx, tp_idx) =
            self.resolver_alvo_predio("fechar_predio", pais_id, provincia_id, tipo_predio_id)?;
        let def_nome = self.catalogo.tipos_predio[tp_idx].nome.clone();
        let dia_atual = self.dias_decorridos;

        let (prov_nome, demitidos, fracao_forca, nivel_restante) = {
            let prov = &mut self.provincias[p_idx];
            let pop_total_prov = prov.populacao_total().max(1.0);
            let predio = prov
                .predios
                .iter_mut()
                .find(|b| b.cache.tipo_idx == tp_idx && b.nivel > 0)
                .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "Provincia",
                    origem_id: provincia_id.to_string(),
                    alvo_entidade: "Predio",
                    alvo_id: tipo_predio_id.to_string(),
                })?;
            if predio.fechado {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "Predio",
                    id: tipo_predio_id.to_string(),
                    detalhe: "o prédio já se encontra fechado".into(),
                });
            }

            let demitidos = predio.total_empregados();
            for slot in predio.cache.vagas_mut() {
                slot.empregados = 0.0;
            }
            predio.empregados_por_tipo.clear();
            predio.fechado = true;
            predio.dias_retomada = 0;
            predio.dias_fechado = 1;
            predio.dias_prejuizo = 0;
            predio.cache.escala_efetiva_ultimo_tick = 0.0;
            predio.cache.fator_insumos_ultimo_tick = 1.0;
            predio.dia_ultima_mudanca_estado = dia_atual;
            let niv = predio.nivel;

            reconciliar_empregados_provincia(
                prov,
                &mut self.buffers.scratch_tipo_a,
                &mut self.buffers.scratch_tipo_b,
            );
            recalcular_vagas_abertas_provincia(prov);
            (
                prov.nome.clone(),
                demitidos,
                (demitidos / pop_total_prov).clamp(0.0, 1.0),
                niv,
            )
        };

        let e_idx = self.provincias[p_idx].estado_idx;
        if e_idx < self.estados.len() {
            atualizar_estado_local(&self.catalogo, &mut self.estados[e_idx], &self.provincias);
        }

        Ok(ResultadoAcaoPredio {
            pais_id: pais_id.to_string(),
            provincia_id: provincia_id.to_string(),
            provincia_nome: prov_nome,
            tipo_predio_id: tipo_predio_id.to_string(),
            tipo_predio_nome: def_nome,
            trabalhadores_demitidos: demitidos,
            fracao_forca_trabalho_provincia: fracao_forca,
            valor_monetario: 0.0,
            nivel_restante,
        })
    }

    /// Reabre um prédio fechado pagando o custo de remobilização (do fundo privado se `usar_fundo_privado`,
    /// ou do tesouro/crédito soberano caso contrário) e iniciando o período de retomada gradual.
    pub fn reabrir_predio(
        &mut self,
        pais_id: &str,
        provincia_id: &str,
        tipo_predio_id: &str,
        usar_fundo_privado: bool,
    ) -> ResultadoEconomia<ResultadoAcaoPredio> {
        let (c_idx, p_idx, tp_idx) =
            self.resolver_alvo_predio("reabrir_predio", pais_id, provincia_id, tipo_predio_id)?;

        let esta_fechado = self.provincias[p_idx]
            .predios
            .iter()
            .find(|b| b.cache.tipo_idx == tp_idx && b.nivel > 0)
            .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                origem_entidade: "Provincia",
                origem_id: provincia_id.to_string(),
                alvo_entidade: "Predio",
                alvo_id: tipo_predio_id.to_string(),
            })?
            .fechado;
        if !esta_fechado {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "Predio",
                id: tipo_predio_id.to_string(),
                detalhe: "o prédio já está aberto".into(),
            });
        }

        let custo = self.custo_reabrir_predio(pais_id, provincia_id, tipo_predio_id)?;

        // Cobrar o custo de remobilização mantendo as invariantes fiscais (`tesouro >= 0`, `fundo >= 0`)
        if usar_fundo_privado {
            if self.paises[c_idx].fundo_investimento_privado + 1e-6 < custo {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "Pais",
                    id: pais_id.to_string(),
                    detalhe: "fundo de investimento privado insuficiente para reabrir o prédio"
                        .into(),
                });
            }
            self.paises[c_idx].fundo_investimento_privado =
                (self.paises[c_idx].fundo_investimento_privado - custo).max(0.0);
        } else {
            let orc = &mut self.paises[c_idx].orcamento;
            if orc.tesouro >= custo {
                orc.tesouro -= custo;
            } else {
                let falta = custo - orc.tesouro.max(0.0);
                if orc.em_calote || orc.divida + falta > orc.limite_credito {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "Pais",
                        id: pais_id.to_string(),
                        detalhe: "tesouro ou limite de crédito insuficiente para reabrir o prédio"
                            .into(),
                    });
                }
                orc.tesouro = 0.0;
                orc.divida += falta;
            }
        }

        let def_nome = self.catalogo.tipos_predio[tp_idx].nome.clone();
        let dia_atual = self.dias_decorridos;

        // Mobilização inicial: contrata até `fator_retomada` (25%) dos trabalhadores desempregados locais compatíveis
        let (prov_nome, nivel_restante) = {
            let prov = &mut self.provincias[p_idx];
            self.buffers.scratch_tipo_c.fill(0.0);
            for pop in &prov.pops {
                if pop.populacao > 0.0 {
                    self.buffers.scratch_tipo_c[pop.tipo_idx] += pop.desempregados();
                }
            }

            let predio = prov
                .predios
                .iter_mut()
                .find(|b| b.cache.tipo_idx == tp_idx && b.nivel > 0)
                .expect("prédio validado");
            predio.fechado = false;
            predio.dias_retomada = Predio::TEMPO_RETOMADA_DIAS;
            predio.dias_fechado = 0;
            predio.dias_prejuizo = 0;
            predio.dia_ultima_mudanca_estado = dia_atual;

            let cap_inicial = (predio.nivel as f64) * predio.fator_retomada();
            for slot in predio.cache.vagas_mut() {
                let tp_i = slot.tipo_pop_idx as usize;
                let meta = (slot.vagas_por_nivel as f64) * cap_inicial;
                let disp = self.buffers.scratch_tipo_c[tp_i];
                let contrata = meta.min(disp);
                slot.empregados = contrata;
                self.buffers.scratch_tipo_c[tp_i] = (disp - contrata).max(0.0);
            }
            sincronizar_mapa_empregados(&self.catalogo, predio);
            let niv = predio.nivel;

            reconciliar_empregados_provincia(
                prov,
                &mut self.buffers.scratch_tipo_a,
                &mut self.buffers.scratch_tipo_b,
            );
            recalcular_vagas_abertas_provincia(prov);
            (prov.nome.clone(), niv)
        };

        let e_idx = self.provincias[p_idx].estado_idx;
        if e_idx < self.estados.len() {
            atualizar_estado_local(&self.catalogo, &mut self.estados[e_idx], &self.provincias);
        }

        Ok(ResultadoAcaoPredio {
            pais_id: pais_id.to_string(),
            provincia_id: provincia_id.to_string(),
            provincia_nome: prov_nome,
            tipo_predio_id: tipo_predio_id.to_string(),
            tipo_predio_nome: def_nome,
            trabalhadores_demitidos: 0.0,
            fracao_forca_trabalho_provincia: 0.0,
            valor_monetario: custo,
            nivel_restante,
        })
    }

    /// Demole `niveis_remover` níveis de um prédio existente, liberando espaço regional (terra arável,
    /// cotas de jazidas minerais, índices estaduais e infraestrutura) e devolvendo parte pequena do valor.
    pub fn demolir_predio(
        &mut self,
        pais_id: &str,
        provincia_id: &str,
        tipo_predio_id: &str,
        niveis_remover: u32,
        creditar_fundo_privado: bool,
    ) -> ResultadoEconomia<ResultadoAcaoPredio> {
        let (c_idx, p_idx, tp_idx) =
            self.resolver_alvo_predio("demolir_predio", pais_id, provincia_id, tipo_predio_id)?;
        let reembolso_materiais =
            self.reembolso_demolir_predio(pais_id, provincia_id, tipo_predio_id, niveis_remover)?;

        let def_nome = self.catalogo.tipos_predio[tp_idx].nome.clone();
        let dia_atual = self.dias_decorridos;

        let (prov_nome, demitidos, fracao_forca, nivel_restante, caixa_liberado) = {
            let prov = &mut self.provincias[p_idx];
            let pop_total_prov = prov.populacao_total().max(1.0);
            let pos = prov
                .predios
                .iter()
                .position(|b| b.cache.tipo_idx == tp_idx && b.nivel > 0)
                .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "Provincia",
                    origem_id: provincia_id.to_string(),
                    alvo_entidade: "Predio",
                    alvo_id: tipo_predio_id.to_string(),
                })?;

            let removidos = niveis_remover.max(1).min(prov.predios[pos].nivel);
            let novo_nivel = prov.predios[pos].nivel - removidos;

            let (demitidos, caixa_extra) = if novo_nivel == 0 {
                let b = prov.predios.remove(pos);
                (b.total_empregados(), b.reservas_caixa.max(0.0))
            } else {
                let b = &mut prov.predios[pos];
                b.nivel = novo_nivel;
                b.dia_ultima_mudanca_estado = dia_atual;
                let cap_vagas = (novo_nivel as f64) * b.fator_retomada();
                let mut desligados = 0.0;
                for slot in b.cache.vagas_mut() {
                    let teto = (slot.vagas_por_nivel as f64) * cap_vagas;
                    if slot.empregados > teto {
                        desligados += slot.empregados - teto;
                        slot.empregados = teto;
                    }
                }
                let teto_caixa = (novo_nivel as f64) * RESERVA_CAIXA_POR_NIVEL;
                let extra = (b.reservas_caixa - teto_caixa).max(0.0);
                b.reservas_caixa = b.reservas_caixa.min(teto_caixa);
                sincronizar_mapa_empregados(&self.catalogo, b);
                (desligados, extra)
            };

            reconciliar_empregados_provincia(
                prov,
                &mut self.buffers.scratch_tipo_a,
                &mut self.buffers.scratch_tipo_b,
            );
            atualizar_terra_aravel_ocupada(&self.catalogo, prov);
            recalcular_vagas_abertas_provincia(prov);

            (
                prov.nome.clone(),
                demitidos,
                (demitidos / pop_total_prov).clamp(0.0, 1.0),
                novo_nivel,
                caixa_extra,
            )
        };

        let credito_total = reembolso_materiais + caixa_liberado;
        if creditar_fundo_privado {
            self.paises[c_idx].fundo_investimento_privado += credito_total;
        } else {
            self.paises[c_idx].orcamento.tesouro += credito_total;
        }

        let e_idx = self.provincias[p_idx].estado_idx;
        if e_idx < self.estados.len() {
            atualizar_estado_local(&self.catalogo, &mut self.estados[e_idx], &self.provincias);
        }

        Ok(ResultadoAcaoPredio {
            pais_id: pais_id.to_string(),
            provincia_id: provincia_id.to_string(),
            provincia_nome: prov_nome,
            tipo_predio_id: tipo_predio_id.to_string(),
            tipo_predio_nome: def_nome,
            trabalhadores_demitidos: demitidos,
            fracao_forca_trabalho_provincia: fracao_forca,
            valor_monetario: reembolso_materiais,
            nivel_restante,
        })
    }

    /// Avalia os prédios de um país com histerese e fecha os que acumulam prejuízo persistente,
    /// reabre os que voltaram a ser sólidos ou demole 1 nível dos que estão fechados há muito tempo
    /// sem perspectiva de retorno. Retorna `(fechados, reabertos, demolidos)`.
    ///
    /// Complexidade: O(prédios do país), executado no máximo 1 vez por mês por país,
    /// limitando as ações a no máximo 2 por chamada.
    pub fn gerir_predios_ia_pais(&mut self, pais_id: &str) -> (u32, u32, u32) {
        let Some(&c_idx) = self.indice_paises.get(pais_id) else {
            return (0, 0, 0);
        };
        let dia_atual = self.dias_decorridos;
        let m_idx = self.paises[c_idx].mercado_idx;
        let exige_estatal = self.paises[c_idx].exige_propriedade_estatal_lei;
        let em_calote = self.paises[c_idx].orcamento.em_calote;
        let tesouro_disp = self.paises[c_idx].orcamento.tesouro;
        let fundo_priv_disp = self.paises[c_idx].fundo_investimento_privado;

        // Coleta até 2 ações candidatas em uma única passagem O(prédios do país)
        #[derive(Clone)]
        enum AcaoIa {
            Fechar { prov_id: String, tipo_id: String },
            Reabrir { prov_id: String, tipo_id: String, privado: bool },
            Demolir { prov_id: String, tipo_id: String, privado: bool },
        }

        let mut acoes: Vec<AcaoIa> = Vec::new();
        for &p_idx in &self.paises[c_idx].provincias_indices {
            if acoes.len() >= 2 {
                break;
            }
            let prov = &self.provincias[p_idx];
            let estado = &self.estados[prov.estado_idx];
            for b in &prov.predios {
                if acoes.len() >= 2 {
                    break;
                }
                if !predio_elegivel_gestao_automatica(&self.catalogo, b) {
                    continue;
                }
                if b.dia_ultima_mudanca_estado > 0
                    && dia_atual < b.dia_ultima_mudanca_estado + HISTERESE_MUDANCA_PREDIO_DIAS
                {
                    continue;
                }

                let eh_privado = !exige_estatal
                    && b.cache.regime_propriedade != crate::tipos::RegimePropriedade::Estatal;
                let margem_est = estimar_margem_operacional_predio(
                    &self.catalogo,
                    &self.mercados[m_idx],
                    estado,
                    &self.paises[c_idx],
                    b,
                );

                if !b.fechado {
                    let margem_atual = if b.escala_efetiva() > 0.0 {
                        b.margem_lucro().min(margem_est)
                    } else {
                        margem_est
                    };
                    if b.dias_retomada == 0
                        && b.dias_prejuizo >= DIAS_PREJUIZO_FECHAR_IA
                        && margem_atual < -0.10
                        && b.lucro_ultimo_tick <= 0.0
                        && b.reservas_caixa < (b.nivel as f64) * 350.0
                    {
                        acoes.push(AcaoIa::Fechar {
                            prov_id: prov.id.clone(),
                            tipo_id: b.tipo_predio_id.clone(),
                        });
                    }
                } else if b.dias_fechado >= DIAS_FECHADO_REABRIR_IA && margem_est >= 0.12 {
                    let def = &self.catalogo.tipos_predio[b.cache.tipo_idx];
                    let custo = calcular_custo_reabertura_por_precos(
                        &self.catalogo,
                        def,
                        b.nivel,
                        |bem_id| {
                            self.catalogo
                                .indice_bens
                                .get(bem_id)
                                .map_or(20.0, |&g| self.mercados[m_idx].precos_idx[g])
                        },
                    );
                    let pode_pagar = if eh_privado {
                        fundo_priv_disp >= custo * 1.3
                    } else {
                        !em_calote && tesouro_disp >= custo * 1.2
                    };
                    if pode_pagar {
                        acoes.push(AcaoIa::Reabrir {
                            prov_id: prov.id.clone(),
                            tipo_id: b.tipo_predio_id.clone(),
                            privado: eh_privado,
                        });
                    }
                } else if b.dias_fechado >= DIAS_FECHADO_DEMOLIR_IA && margem_est < -0.18 {
                    acoes.push(AcaoIa::Demolir {
                        prov_id: prov.id.clone(),
                        tipo_id: b.tipo_predio_id.clone(),
                        privado: eh_privado,
                    });
                }
            }
        }

        let (mut fechados, mut reabertos, mut demolidos) = (0u32, 0u32, 0u32);
        for acao in acoes {
            match acao {
                AcaoIa::Fechar { prov_id, tipo_id } => {
                    if self.fechar_predio(pais_id, &prov_id, &tipo_id).is_ok() {
                        fechados += 1;
                    }
                }
                AcaoIa::Reabrir {
                    prov_id,
                    tipo_id,
                    privado,
                } => {
                    if self
                        .reabrir_predio(pais_id, &prov_id, &tipo_id, privado)
                        .is_ok()
                    {
                        reabertos += 1;
                    }
                }
                AcaoIa::Demolir {
                    prov_id,
                    tipo_id,
                    privado,
                } => {
                    if self
                        .demolir_predio(pais_id, &prov_id, &tipo_id, 1, privado)
                        .is_ok()
                    {
                        demolidos += 1;
                    }
                }
            }
        }
        (fechados, reabertos, demolidos)
    }
}

/// Verifica se um prédio pode ser fechado/reaberto/demolido automaticamente pela IA ou setor privado.
/// Protege prédios governamentais, subsidiados, militares, de construção, de infraestrutura e
/// alimentos básicos essenciais (exceto em colapso severo prolongado).
pub fn predio_elegivel_gestao_automatica(catalogo: &CatalogoEconomia, b: &Predio) -> bool {
    if b.nivel == 0 || b.cache.financiado_pelo_governo || b.subsidiado {
        return false;
    }
    let Some(def) = catalogo.tipos_predio.get(b.cache.tipo_idx) else {
        return false;
    };
    if matches!(
        def.categoria,
        crate::tipos::CategoriaPredio::Infraestrutura
            | crate::tipos::CategoriaPredio::Construcao
            | crate::tipos::CategoriaPredio::Governamental
            | crate::tipos::CategoriaPredio::Militar
    ) {
        return false;
    }
    if matches!(
        def.id.as_str(),
        "fazenda_graos" | "rancho_pecuario" | "pecuaria_pesca" | "porto_pesca" | "centro_urbano"
    ) && !b.fechado
        && b.dias_prejuizo < 240
    {
        return false;
    }
    true
}

/// Passo mensal enxuto do setor privado autônomo (executado em `governo.rs` a cada 30 dias por país):
/// fecha instalações privadas com prejuízo persistente, reabre quando voltam a dar margem >= 12%
/// e demole 1 nível das instalações fechadas há mais de 270 dias sem viabilidade.
pub(crate) fn gerir_predios_setor_privado_pais(
    catalogo: &CatalogoEconomia,
    mercado: &Mercado,
    estados: &[Estado],
    pais: &mut Pais,
    provincias: &mut [Provincia],
    dias_decorridos: u64,
) {
    if pais.exige_propriedade_estatal_lei {
        return;
    }
    let mut acoes_restantes = 2usize;
    let mut scratch_a = [0.0_f64; 32];
    let mut scratch_b = [0.0_f64; 32];
    let mut scratch_c = [0.0_f64; 32];
    let n_tp = catalogo.tipos_pop.len().min(32);

    for &p_idx in &pais.provincias_indices {
        if acoes_restantes == 0 {
            break;
        }
        let e_idx = provincias[p_idx].estado_idx;
        if e_idx >= estados.len() {
            continue;
        }
        let estado = &estados[e_idx];
        let mut mudou_prov = false;
        let mut remover_pos: Option<usize> = None;

        for (pos, b) in provincias[p_idx].predios.iter_mut().enumerate() {
            if acoes_restantes == 0 {
                break;
            }
            if !predio_elegivel_gestao_automatica(catalogo, b)
                || b.cache.regime_propriedade == crate::tipos::RegimePropriedade::Estatal
            {
                continue;
            }
            if b.dia_ultima_mudanca_estado > 0
                && dias_decorridos < b.dia_ultima_mudanca_estado + HISTERESE_MUDANCA_PREDIO_DIAS
            {
                continue;
            }

            let margem_est =
                estimar_margem_operacional_predio(catalogo, mercado, estado, pais, b);

            if !b.fechado {
                let margem_atual = if b.escala_efetiva() > 0.0 {
                    b.margem_lucro().min(margem_est)
                } else {
                    margem_est
                };
                if b.dias_retomada == 0
                    && b.dias_prejuizo >= DIAS_PREJUIZO_FECHAR_IA
                    && margem_atual < -0.10
                    && b.lucro_ultimo_tick <= 0.0
                    && b.reservas_caixa < (b.nivel as f64) * 350.0
                {
                    for slot in b.cache.vagas_mut() {
                        slot.empregados = 0.0;
                    }
                    b.empregados_por_tipo.clear();
                    b.fechado = true;
                    b.dias_retomada = 0;
                    b.dias_fechado = 1;
                    b.dias_prejuizo = 0;
                    b.cache.escala_efetiva_ultimo_tick = 0.0;
                    b.dia_ultima_mudanca_estado = dias_decorridos;
                    mudou_prov = true;
                    acoes_restantes -= 1;
                }
            } else if b.dias_fechado >= DIAS_FECHADO_REABRIR_IA && margem_est >= 0.12 {
                let def = &catalogo.tipos_predio[b.cache.tipo_idx];
                let custo =
                    calcular_custo_reabertura_por_precos(catalogo, def, b.nivel, |bem_id| {
                        catalogo
                            .indice_bens
                            .get(bem_id)
                            .map_or(20.0, |&g| mercado.precos_idx[g])
                    });
                if pais.fundo_investimento_privado >= custo * 1.4 {
                    pais.fundo_investimento_privado =
                        (pais.fundo_investimento_privado - custo).max(0.0);
                    b.fechado = false;
                    b.dias_retomada = Predio::TEMPO_RETOMADA_DIAS;
                    b.dias_fechado = 0;
                    b.dias_prejuizo = 0;
                    b.dia_ultima_mudanca_estado = dias_decorridos;
                    mudou_prov = true;
                    acoes_restantes -= 1;
                }
            } else if b.dias_fechado >= DIAS_FECHADO_DEMOLIR_IA && margem_est < -0.18 {
                let def = &catalogo.tipos_predio[b.cache.tipo_idx];
                let reembolso =
                    calcular_reembolso_demolicao_por_precos(catalogo, def, 1, |bem_id| {
                        catalogo
                            .indice_bens
                            .get(bem_id)
                            .map_or(20.0, |&g| mercado.precos_idx[g])
                    });
                pais.fundo_investimento_privado += reembolso;
                if b.nivel <= 1 {
                    pais.fundo_investimento_privado += b.reservas_caixa.max(0.0);
                    remover_pos = Some(pos);
                } else {
                    b.nivel -= 1;
                    b.dia_ultima_mudanca_estado = dias_decorridos;
                    let teto_caixa = (b.nivel as f64) * RESERVA_CAIXA_POR_NIVEL;
                    if b.reservas_caixa > teto_caixa {
                        pais.fundo_investimento_privado += b.reservas_caixa - teto_caixa;
                        b.reservas_caixa = teto_caixa;
                    }
                }
                mudou_prov = true;
                acoes_restantes -= 1;
                break;
            }
        }

        if let Some(pos) = remover_pos {
            provincias[p_idx].predios.remove(pos);
        }
        if mudou_prov {
            let prov = &mut provincias[p_idx];
            // Se algum prédio acabou de reabrir, aloca até 25% das vagas dos desempregados locais
            scratch_c[..n_tp].fill(0.0);
            for pop in &prov.pops {
                if pop.populacao > 0.0 && pop.tipo_idx < n_tp {
                    scratch_c[pop.tipo_idx] += pop.desempregados();
                }
            }
            for b in &mut prov.predios {
                if !b.fechado
                    && b.dias_retomada == Predio::TEMPO_RETOMADA_DIAS
                    && b.total_empregados() <= 0.0
                {
                    let cap_ini = (b.nivel as f64) * b.fator_retomada();
                    for slot in b.cache.vagas_mut() {
                        let tp_i = slot.tipo_pop_idx as usize;
                        if tp_i < n_tp {
                            let meta = (slot.vagas_por_nivel as f64) * cap_ini;
                            let contrata = meta.min(scratch_c[tp_i]);
                            slot.empregados = contrata;
                            scratch_c[tp_i] = (scratch_c[tp_i] - contrata).max(0.0);
                        }
                    }
                    sincronizar_mapa_empregados(catalogo, b);
                }
            }
            reconciliar_empregados_provincia(prov, &mut scratch_a[..n_tp], &mut scratch_b[..n_tp]);
            atualizar_terra_aravel_ocupada(catalogo, prov);
            recalcular_vagas_abertas_provincia(prov);
        }
    }
}
