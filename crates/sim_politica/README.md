# sim_politica

Biblioteca Rust pura (sem Bevy) com o motor político estilo Victoria 3 para o jogo Guerra Fria (1945+).

- `src/catalogo.rs`: ideologias, leis (por categoria), grupos de interesse e arquétipos de partido (dados estáticos, RON).
- `src/entidades/`: estado dinâmico (pops, GIs, líderes, partidos, tramitação de lei, eleição, movimentos, crises, influência externa, `PaisPolitica`).
- `src/motor/`: tick diário (`MundoPolitico::executar_tick_diario`): influência externa, clout, radicalismo, aprovação, partidos, eleições, legitimidade, legislativo, movimentos e crises.
- `CenarioPolitica::carregar_diretorio(dir)` lê `catalogo_politica.ron` (ou `ideologias.ron`, `leis.ron`, `grupos_interesse.ron`) e `paises_politica.ron` (lista de `PaisPolitica`).
- Ids em snake_case ASCII; tags de país com 3 letras maiúsculas; tudo determinístico por semente.
- Guerra civil sai como `SinalGuerraCivil` via `MundoPolitico::drenar_eventos_guerra_civil()` para o futuro `sim_guerra`.
- Testes: `tests/integracao.rs` (fixtures construídas em código): 20 anos de democracia e de partido único, leis, eleições, determinismo, radicalismo, influência externa.

Rodar: `cargo test && cargo clippy --all-targets && cargo fmt --check`.
