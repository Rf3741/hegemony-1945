# sim_territorio

Crate Rust puro (sem Bevy) do Guerra Fria (1945+): território, urbanização, eras visuais, infraestrutura e descolonização.

## Módulos
- `entidades`: `Regiao` (chave `adm1_code`) -> `Estado` -> `RegiaoEstrategica`; `PaisTerritorial` (tag de 3 letras); núcleos e reivindicações no `Estado`.
- `transferencia`: transferência de soberania com efeitos nas pops (cultura aceita/discriminada, migração, expulsão).
- `urbanizacao`: `EstadoUrbano` (índices suavizados, `NivelCidade` 1-8, `NivelIndustrial` 0-8 com histerese), `Cidade`, `DescritorVisualEstado`.
- `eras`: era visual por país e por estado (`era1945` ... `era1985`) a partir de tecnologias e métodos.
- `infraestrutura`: estradas, ferrovias, portos, aeroportos e rede elétrica (capacidade x uso).
- `descolonizacao`: fervor anticolonial, independência negociada, partição, secessão armada.
- `mundo`: `MundoTerritorial` (índices O(1), `avancar_dia`, `construir_predio`, `desbloquear_tecnologia`, `transferir_estado_operacao`, `conceder_independencia_operacao`).

## Dados RON (`tests/fixtures`)
- `culturas.ron`: `(id, nome, grupo_linguistico, heranca_cultural)`.
- `predios_urbanos.ron`: `DefinicaoPredioUrbano` (pontos de urbanização, empregos, demandas de infraestrutura por nível).
- `tecnologias_eras.ron`: `(id, nome, era, pilar, peso_visual, descricao)`.
- `metodos_eras.ron`: `(metodo_id, nome, era_minima, peso_adocao)`.
- Cenário (`CenarioTerritorio::carregar_diretorio`): `regioes_estrategicas.ron`, `paises.ron`, `estados.ron`, `regioes.ron`, `movimentos_independencia.ron`.

Ids em snake_case ASCII; textos em pt-BR.

## Uso visual
Ler `Estado::desenvolvimento_urbano.nivel_cidade` / `nivel_industrial` e `Cidade::{nivel, era_visual, tamanho_visual}` para desenhar cidades e fábricas; `descritor_visual` entrega densidades, iluminação noturna e fumaça (0..1).

## Testes
`cargo test` (unitários + `tests/integracao.rs`, incluindo 30 anos de industrialização com nível de cidade suave).
