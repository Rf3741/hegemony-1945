# Guerra Fria (1945-1991)

Jogo de grande estratégia geopolítica, macroeconômica e sociopolítica em tempo real pausável, para Linux e Windows, feito em **Rust + Bevy 0.20 + `bevy_egui`**. Começa em 1º de janeiro de 1945 e inspira-se na profundidade sistêmica de *Victoria 3*. Tudo em português brasileiro.

O design completo (pilares, economia, política, guerra, roadmap) está em [DESIGN.md](DESIGN.md).

## Como rodar

Requisitos: Rust estável (edição 2024) e, no Linux, as bibliotecas do Bevy (`pkg-config`, `libasound2-dev`, `libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`; veja [BUILD.md](BUILD.md) para outras distros).

```bash
cargo run --release -p guerra_fria      # o jogo (rode a partir da raiz do repositório)
cargo run -p gerador_mapa               # ferramenta de dados do mapa
cargo test --workspace --exclude guerra_fria   # testes (sem GPU)
```

Atalhos: `make help` ou `just` listam todas as receitas (`check`, `fmt`, `clippy`, `test`, `run-fast`, `package-linux`, `cross-windows-xwin`...). Detalhes de build, linkers rápidos, cross-compile para Windows e CI em [BUILD.md](BUILD.md).

## Estrutura do workspace

| Caminho | Função |
| :--- | :--- |
| `crates/guerra_fria` | Cliente Bevy + egui (mapa político, câmera, UI, plugins) |
| `crates/gerador_mapa` | CLI de geração/validação do mapa e cenário de 1945 |
| `crates/simulacao` | Motor de simulação legado (sem Bevy) |
| `crates/sim_economia` | Economia: prédios, pops, mercados, preços |
| `crates/sim_politica` | Política: grupos de interesse, leis, legitimidade |
| `crates/sim_territorio` | Territórios, províncias, soberania |
| `crates/sim_guerra` | Guerra e unidades militares |
| `crates/bandeiras_resolver` | Resolução de bandeiras por país/regime |
| `assets/` | Mapa de províncias, shaders |
| `data/` | Cenário 1945 em RON (países, donos de província, regras) |
| `scripts/` | Cross-compile para Windows e validação dos workflows |
| `.github/workflows/` | CI (fmt, clippy, test, build) e release (tag `v*`) |

## Releases

Ao criar uma tag `v*` (ex.: `v0.1.0`), o GitHub Actions publica `guerra-fria-v0.1.0-linux-x86_64.tar.gz` e `guerra-fria-v0.1.0-windows-x86_64.zip`, ambos com `assets/` e `data/`.

## Licença

MIT OU Apache-2.0.
