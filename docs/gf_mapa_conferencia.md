# Conferencia de adm1_code (T04)

Gerado por `cargo test -p gf_mapa --test conferencia` com `GF_ESCREVER_RELATORIO=1`.

## Totais

- `assets/map/provinces.ron`: 4596 provincias
- `regioes_1945_canonico.ron`: 4596 regioes
- `territorio_dados/estados.ron`: 930 estados, 4596 adm1 referenciados

## Divergencias

### regioes_1945 ausentes em provinces.ron (esperado 0): 0


### estados.ron ausentes em provinces.ron (esperado 0): 0


### regioes_1945 nao cobertas por nenhum estado: 0


### provinces.ron sem regiao em regioes_1945: 0


## Conclusao

Se as duas primeiras listas estao em 0, `provinces.ron` usa os mesmos `adm1_code` do Natural Earth que os dados de 1945; nao e necessaria tabela `id_u32 -> adm1_code` separada (o proprio `provinces.ron` ja e essa tabela: `id` -> `adm1_code`).
