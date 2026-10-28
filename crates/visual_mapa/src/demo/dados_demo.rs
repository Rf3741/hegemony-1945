//! Dados FALSOS de demonstração (cidades, instalações, conexões de `assets/visual/demo/*.ron`)
//! convertidos para a interface [`EntradaVisual`]. Leitura em thread (industrias.ron tem ~8 MB).

use super::tempo::TempoSimulacao;
use crate::config::VisualMapaConfig;
use crate::dados::ler_ron;
use crate::entrada::{CidadeEntrada, ConexaoEntrada, EntradaVisual, InstalacaoEntrada};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, TaskPool, block_on, futures_lite::future};
pub use crate::dados::{CidadeDados, ConexaoDados, IndustriaDados};

/// Conteúdo bruto lido do disco.
#[derive(Default)]
pub struct DadosBrutos {
    pub cidades: Vec<CidadeDados>,
    pub conexoes: Vec<ConexaoDados>,
    pub industrias: Vec<IndustriaDados>,
}

#[derive(Resource)]
pub struct TarefaDados(Task<DadosBrutos>);

/// Dados carregados, prontos para a interpolação por ano.
#[derive(Resource)]
pub struct DadosDemo(pub DadosBrutos);

/// Fator de evolução 0..=1 entre 1945 e 1991.
pub fn fator_evolucao(ano: f32) -> f32 {
    ((ano - TempoSimulacao::ANO_INICIAL) / (TempoSimulacao::ANO_FINAL - TempoSimulacao::ANO_INICIAL)).clamp(0.0, 1.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Converte os dados falsos em entradas no ano dado (função pura, testável).
pub fn montar_entrada(d: &DadosBrutos, ano: f32, limite_instalacoes: usize) -> EntradaVisual {
    let t = fator_evolucao(ano);
    let ano_i = ano.floor() as u16;
    EntradaVisual {
        cidades: d
            .cidades
            .iter()
            .map(|c| CidadeEntrada {
                id: c.id.clone(),
                nome: c.nome.clone(),
                pais_tag: c.pais_tag.clone(),
                posicao: c.posicao,
                nivel: lerp(c.nivel_1945 as f32, c.nivel_1991 as f32, t),
                populacao: lerp(c.populacao_1945 as f32, c.populacao_1991 as f32, t) as u32,
                eh_capital: c.eh_capital,
                nivel_maximo: Some(c.nivel_1991),
                ..default()
            })
            .collect(),
        instalacoes: d
            .industrias
            .iter()
            .take(limite_instalacoes)
            .map(|i| InstalacaoEntrada {
                id: i.id.clone(),
                nome: i.nome.clone(),
                tipo: i.categoria.clone(),
                pais_tag: i.pais_tag.clone(),
                posicao: i.posicao,
                nivel: lerp(i.nivel_inicial as f32, i.nivel_maximo as f32, t).round() as u8,
                producao: 0.85 + (ano_i as f32 * 0.1).sin() * 0.15,
                poluicao: i.taxa_fumaca,
                ativa: ano_i >= i.ano_construcao,
                era: None,
            })
            .collect(),
        conexoes: d
            .conexoes
            .iter()
            .map(|c| ConexaoEntrada {
                id: c.id.clone(),
                origem_id: c.origem_id.clone(),
                destino_id: c.destino_id.clone(),
                tipo: c.tipo.clone(),
                nivel: lerp(c.nivel_1945 as f32, c.nivel_1991 as f32, t).round() as u8,
                ativa: ano_i >= c.ano_construcao,
            })
            .collect(),
    }
}

/// Dispara a leitura dos RON em outra thread.
pub fn iniciar_carga_sistema(mut commands: Commands, cfg: Res<VisualMapaConfig>) {
    let dir = cfg.raiz_dados.join("demo");
    let tarefa = AsyncComputeTaskPool::get_or_init(TaskPool::default).spawn(async move {
        DadosBrutos {
            cidades: ler_ron(&dir.join("cidades.ron")).unwrap_or_default(),
            conexoes: ler_ron(&dir.join("conexoes.ron")).unwrap_or_default(),
            industrias: ler_ron(&dir.join("industrias.ron")).unwrap_or_default(),
        }
    });
    commands.insert_resource(TarefaDados(tarefa));
}

/// Recolhe a tarefa quando termina.
pub fn aguardar_carga_sistema(mut commands: Commands, tarefa: Option<ResMut<TarefaDados>>) {
    let Some(mut t) = tarefa else { return };
    if let Some(d) = block_on(future::poll_once(&mut t.0)) {
        info!("demo: {} cidades, {} instalações, {} conexões", d.cidades.len(), d.industrias.len(), d.conexoes.len());
        commands.remove_resource::<TarefaDados>();
        commands.insert_resource(DadosDemo(d));
    }
}

/// Atualiza a [`EntradaVisual`] quando o ano avançou o bastante (~2 dias de jogo por 0,01 ano).
pub fn alimentar_entrada_sistema(
    dados: Option<Res<DadosDemo>>,
    tempo: Res<TempoSimulacao>,
    cfg: Res<VisualMapaConfig>,
    mut entrada: ResMut<EntradaVisual>,
    mut ultimo_ano: Local<Option<f32>>,
) {
    let Some(dados) = dados else { return };
    if ultimo_ano.is_some_and(|u| (u - tempo.ano_continuo).abs() < 0.02) {
        return;
    }
    *ultimo_ano = Some(tempo.ano_continuo);
    *entrada = montar_entrada(&dados.0, tempo.ano_continuo, cfg.limite_instalacoes);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dados() -> DadosBrutos {
        DadosBrutos {
            cidades: vec![CidadeDados {
                id: "x".into(), nome: "X".into(), pais_tag: "BRA".into(), adm1_code: "BRA-1".into(),
                posicao: (-46.0, -23.0), populacao_1945: 1000, populacao_1991: 3000,
                nivel_1945: 2, nivel_1991: 6, eh_capital: false,
            }],
            conexoes: vec![],
            industrias: vec![IndustriaDados {
                id: "i".into(), nome: "I".into(), categoria: "fabrica".into(), pais_tag: "BRA".into(),
                posicao: (-46.0, -23.0), ano_construcao: 1960, nivel_inicial: 1, nivel_maximo: 3, taxa_fumaca: 0.5,
            }],
        }
    }

    #[test]
    fn nivel_cresce_com_o_ano() {
        let d = dados();
        let a = montar_entrada(&d, 1945.0, 10);
        let b = montar_entrada(&d, 1991.0, 10);
        assert_eq!(a.cidades[0].nivel, 2.0);
        assert_eq!(b.cidades[0].nivel, 6.0);
        assert_eq!(b.cidades[0].populacao, 3000);
    }

    #[test]
    fn instalacao_ativa_so_apos_construcao() {
        let d = dados();
        assert!(!montar_entrada(&d, 1950.0, 10).instalacoes[0].ativa);
        assert!(montar_entrada(&d, 1960.5, 10).instalacoes[0].ativa);
    }

    #[test]
    fn arquivos_demo_parseiam() {
        let dir = crate::dados::raiz_dados_padrao().join("demo");
        let c: Vec<CidadeDados> = ler_ron(&dir.join("cidades.ron")).unwrap();
        assert!(c.len() > 100);
        let k: Vec<ConexaoDados> = ler_ron(&dir.join("conexoes.ron")).unwrap();
        assert!(!k.is_empty());
    }
}
