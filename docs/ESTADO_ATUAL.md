# Estado atual do Hegemony: 1945 (branch `jogavel`)

Rust + Bevy 0.20, jogo de grande estratégia estilo Victoria 3, começando em 1945.

## O que o jogo tem
- Mapa com 7.740 províncias (provinces.png, provinces.ron, province_owners_1945.ron), dono/controlador por província, estados (adm1) e países; relevo sombreado, oceano com brilho de costa e bordas país/estado/província.
- Economia (sim_economia): balanço corrigido, infraestrutura, setor privado, canais de comércio e pedágio; comércio internacional.
- Construção de prédios, rotas, cidades, tráfego e visual de prédios/cidades (crate visual_mapa), estilo do mapa.
- Política (sim_politica + politica_ui): partidos/leis do país do jogador.
- Eventos (crate eventos, jornal), diplomacia (sim_diplomacia, diplomacia_ui) e tecnologia (sim_tecnologia, tecnologia_ui).
- Tempo: pausa e velocidades; rótulos de países; áudio.

## Teclas
- F1: painel do país
- F2: mercado
- F3: comércio internacional
- F4: diplomacia
- F5: tecnologia
- B: abre/fecha construção
- V: alterna a camada visual de construções/cidades (LOD)
- N: alterna dia/noite (luzes das cidades)
- J: jornal de eventos
- Espaço: pausa; 1-5: velocidade do tempo; Esc: menu/pausa
- Câmera: WASD/setas, roda do mouse (zoom), arrastar com botão esquerdo; clique seleciona província

## Ficou de fora / falta
- Guerra (sim_guerra) e território (sim_territorio) não têm interface integrada na partida.
- Sem salvar/carregar, sem IA completa dos países, sem multiplayer.
- Equilíbrio fino da economia, política, eventos e tecnologia ainda não testado em longas partidas.
- Teste visual no jogo (janela) não foi feito durante a integração; só build e testes.
