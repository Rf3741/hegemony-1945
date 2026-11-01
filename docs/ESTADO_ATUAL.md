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
- 1: país · 2: mercado · 3: comércio · 4: diplomacia · 5: tecnologia · 6: guerra · 7: estatísticas · 8: enciclopédia · 9: áudio (linha de cima e teclado numérico; sem F1–F10)
- M / Shift+M: modos de mapa (também pelo seletor); Ctrl+S / Ctrl+L: salvar/carregar rápido
- B: abre/fecha construção
- V: alterna a camada visual de construções/cidades (LOD)
- N: alterna dia/noite (luzes das cidades)
- J: jornal de eventos
- Espaço: pausa; + / −: velocidade do tempo; Esc: menu/pausa
- Câmera: WASD/setas, roda do mouse (zoom), arrastar com botão esquerdo; clique seleciona província

## Ficou de fora / falta
- Integrados em 01/11/2026: população real e turismo, estatísticas, enciclopédia/tutorial, áudio, personagens, eventos/ia_paises, guerra (sim_guerra), descolonização/bandeiras, menu/salvar/carregar/modos de mapa, polimento visual e correção da travada (recálculo incremental).
- Território (sim_territorio) sem interface; sem multiplayer.
- Ctrl+S também aciona S da câmera (move o mapa) enquanto o atalho é usado.
- Equilíbrio fino da economia, política, eventos e tecnologia ainda não testado em longas partidas.
- Teste visual no jogo (janela) não foi feito durante a integração; só build e testes.
