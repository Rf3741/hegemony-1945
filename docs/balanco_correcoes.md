# Correções de balanço de 1945 (branch w-eco)

Sintoma: Brasil/1945 com desemprego 88,7%, calote, saldo negativo e PIB de $4 bi.

Causas e correções:
1. **Infraestrutura zerada**: o cenário não traz ferrovias/rodovias; o uso (2 por nível industrial) superava 20x a capacidade e o `acesso_mercado` caía a 0,05 em todos os estados, matando a produção. `producao::calibrar_infraestrutura_inicial` acrescenta `rede_ferroviaria` (60%) e `rodovia` (40%) até a utilização alvo por riqueza (ricos 0,55-0,70, pobres ~1,0-1,04). A infraestrutura gerada passa a depender do nível (piso de 40% sem tripulação).
2. **Poupança das pops**: com reservas iniciais baixas, o consumo caía a ~10% da renda. Piso de 85% da renda gasta (`consumo.rs`).
3. **Excedente sem comprador**: o comércio mundial residual só cobria déficits; agora o resto do mundo também compra parte do excedente (`global.rs`), com abertura mínima 0,35 mesmo no bloco fechado.
4. **Demissões em cascata**: salário cai proporcional ao prejuízo e só há demissão com salário perto do piso (`trabalho.rs`).
5. **Faixa de preços** 0,5x a 1,5x (`config_simulacao.ron`), reduzindo o aperto de margem.
6. **Métrica de desemprego** exclui camponeses de subsistência (autônomos).

Resultado (1 ano): sem calote nos 10 maiores países; desemprego 6-16%; Brasil PIB ~$14,7 bi (inicial $35 bi), EUA ~$1,44 tri.
Teste: `crates/sim_economia/tests/balanco_1945.rs`. Pendência: Brasil/Itália/China sobem para 19-29% no 3o ano.
