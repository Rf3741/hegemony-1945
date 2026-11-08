//! Ponte entre o crate `personagens` e o jogo: líderes reais de 1945, envelhecimento, morte e
//! sucessão por data, com os traços dos líderes alimentando a política (`sim_politica`).
//!
//! * Os efeitos dos traços dos chefes de Estado e de Governo viram um ajuste de legitimidade
//!   (aplicado uma vez a cada troca de líder, de forma reversível).
//! * Mortes e sucessões do país do jogador aparecem como notificações e no histórico.

use super::jogador::{Notificacoes, PaisJogador};
use super::map::RaizProjeto;
use super::politica_sim::{ComandoPol, PoliticaSim, tag_politica};
use super::time::EventoTickDiario;
use bevy::prelude::*;
use personagens::{Elenco, EventoPersonagem, Papel, Personagem};
use sim_politica::LiderPolitico;
use std::collections::BTreeMap;

/// Estado dos personagens no ECS.
#[derive(Resource)]
pub struct ElencoRes {
    pub elenco: Elenco,
    /// Ajuste de legitimidade já aplicado por país (para aplicar só a diferença).
    aplicado: BTreeMap<String, f64>,
    /// Lista de países já sincronizados com a política (inicial).
    sincronizado: bool,
    /// Mortes e sucessões recentes (texto), mais novas por último.
    pub historico: Vec<String>,
}

pub const INICIO: personagens::Data = (1945, 1, 1);

/// Converte um personagem num líder do motor político.
pub fn para_lider_politico(p: &Personagem, hoje: personagens::Data, elenco: &Elenco) -> LiderPolitico {
    let mut carisma = 1.0;
    for t in &p.tracos {
        if let Some(t) = elenco.traco(t) {
            carisma += t.efeitos.get("apelo_popular").copied().unwrap_or(0.0) * 0.5;
            if t.id.contains("carism") {
                carisma += 0.2;
            }
        }
    }
    let mut l = LiderPolitico::novo(p.id.clone(), p.nome.clone(), p.ideologia.clone(), 60.0, carisma, p.data_nascimento().0 as u16);
    l.tracos = p.tracos.clone();
    let _ = hoje;
    l
}

impl ElencoRes {
    /// Comandos de líder (e de legitimidade) para um país, se algo mudou.
    fn comandos_pais(&mut self, pais: &str, hoje: personagens::Data) -> Vec<ComandoPol> {
        let novo = self.elenco.ajuste_legitimidade(pais);
        let antigo = self.aplicado.insert(pais.to_string(), novo).unwrap_or(0.0);
        let mut delta = novo - antigo;
        let mut v = Vec::new();
        for (papel, estado) in [(Papel::Estado, true), (Papel::Governo, false)] {
            if let Some(p) = self.elenco.lider(pais, papel) {
                v.push(ComandoPol::Lider { tag: pais.to_string(), estado, lider: para_lider_politico(p, hoje, &self.elenco), delta_legitimidade: std::mem::take(&mut delta) });
            }
        }
        v
    }
}

pub struct PersonagensPlugin;

impl Plugin for PersonagensPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, carregar).add_systems(
            Update,
            (sincronizar_inicial, avancar).chain().run_if(resource_exists::<ElencoRes>).run_if(resource_exists::<PoliticaSim>),
        );
        super::personagens_ui::registrar(app);
    }
}

fn carregar(mut commands: Commands, raiz: Res<RaizProjeto>) {
    match Elenco::carregar(&raiz.0.join("assets/data/personagens"), INICIO) {
        Ok(elenco) => commands.insert_resource(ElencoRes { elenco, aplicado: BTreeMap::new(), sincronizado: false, historico: Vec::new() }),
        Err(e) => error!("falha ao carregar os personagens: {e}"),
    }
}

/// Define os líderes de 1945 também para países que surgiram durante a partida.
fn sincronizar_inicial(mut el: ResMut<ElencoRes>, mut pol: ResMut<PoliticaSim>) {
    if pol.nucleo.is_none() {
        return;
    }
    let paises: Vec<String> = pol
        .nucleo
        .as_ref()
        .map(|n| n.mundo.paises().iter().map(|p| p.tag.clone()).collect())
        .unwrap_or_default();
    for p in paises {
        if el.sincronizado && el.aplicado.contains_key(&p) {
            continue;
        }
        for c in el.comandos_pais(&p, INICIO) {
            pol.enviar(c);
        }
    }
    el.sincronizado = true;
}

fn avancar(
    mut leitor: MessageReader<EventoTickDiario>,
    mut el: ResMut<ElencoRes>,
    mut pol: ResMut<PoliticaSim>,
    jogador: Option<Res<PaisJogador>>,
    mut notas: ResMut<Notificacoes>,
    mut ev_jornal: Option<ResMut<super::eventos_ui::EstadoEv>>,
    tempo: Res<Time<Real>>,
) {
    if !el.sincronizado {
        return;
    }
    let meu = jogador.map(|j| tag_politica(&j.0));
    for e in leitor.read() {
        let hoje = (e.nova_data.ano, e.nova_data.mes, e.nova_data.dia);
        let evs = el.elenco.avancar_dia(hoje);
        let mut afetados: Vec<String> = Vec::new();
        for ev in evs {
            let (pais, texto) = match &ev {
                EventoPersonagem::Morte { nome, pais, idade, .. } => (pais.clone(), format!("{nome} morreu aos {idade} anos.")),
                EventoPersonagem::Sucessao { pais, papel, novo, .. } => {
                    let n = el.elenco.personagem(novo).map_or(novo.clone(), |p| p.nome.clone());
                    (pais.clone(), format!("{n} assume como {} ({pais}).", papel.nome()))
                }
            };
            let relevante = matches!(&ev, EventoPersonagem::Sucessao { .. }) || meu.as_deref() == Some(pais.as_str());
            if relevante && (meu.as_deref() == Some(pais.as_str()) || matches!(&ev, EventoPersonagem::Sucessao { papel: Papel::Estado, .. })) {
                el.historico.push(format!("{:02}/{:02}/{}: {texto}", hoje.2, hoje.1, hoje.0));
                if meu.as_deref() == Some(pais.as_str()) {
                    if let Some(j) = ev_jornal.as_deref_mut() {
                        j.estado.jornal.push(eventos::Noticia { data: hoje, texto: texto.clone() });
                        if j.estado.jornal.len() > 200 {
                            j.estado.jornal.remove(0);
                        }
                    }
                    notas.push(texto, tempo.elapsed_secs_f64());
                }
            }
            if matches!(&ev, EventoPersonagem::Sucessao { .. }) && !afetados.contains(&pais) {
                afetados.push(pais);
            }
        }
        if el.historico.len() > 200 {
            el.historico.drain(..100);
        }
        for p in afetados {
            for c in el.comandos_pais(&p, hoje) {
                pol.enviar(c);
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn converte_e_tags_batem_com_politica() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/personagens");
        let e = Elenco::carregar(&dir, INICIO).unwrap();
        let p = e.lider("SOV", Papel::Estado).unwrap();
        let l = para_lider_politico(p, INICIO, &e);
        assert!(l.nome.starts_with("Iosif"));
        assert!(l.carisma >= 0.2);
        // Tags dos personagens existem no cenário político.
        let pol = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/politica/paises_politica.ron")).unwrap();
        let usadas: Vec<&str> = e.paises().into_iter().filter(|t| pol.contains(&format!("tag: \"{t}\""))).collect();
        assert!(usadas.len() >= 45, "só {} tags batem", usadas.len());
    }
}
