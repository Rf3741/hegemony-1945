# ==============================================================================
# justfile - Guerra Fria (1945+)
# Comandos de automação para o workspace Rust + Bevy
# ==============================================================================

set shell := ["bash", "-c"]

versao := `sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml`

# Exibe receitas disponíveis por padrão
default:
    @just --list

# Checa tipos e sintaxe em todo o workspace
check:
    cargo check --workspace --all-targets

# Formata todos os arquivos com rustfmt
fmt:
    cargo fmt --all

# Verifica se a formatação cumpre as regras do rustfmt
fmt-check:
    cargo fmt --all -- --check

# Executa o linter Clippy sem aceitar warnings
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Executa a suíte de testes unitários e de integração
test:
    cargo test --workspace --exclude guerra_fria --verbose

# Compila todo o workspace em modo debug
build:
    cargo build --workspace

# Compila todo o workspace em modo release (LTO ativado)
build-release:
    cargo build --workspace --release

# Compilação rápida para desenvolvimento com dynamic_linking do Bevy
build-fast:
    cargo build --package guerra_fria --features dynamic_linking

# Executa o jogo principal Guerra Fria
run:
    cargo run --package guerra_fria

# Executa o jogo principal com dynamic_linking para iteração rápida
run-fast:
    cargo run --package guerra_fria --features dynamic_linking

# Executa a ferramenta CLI de mapa
run-map-gen:
    cargo run --package gerador_mapa

# Cross-compila para Windows usando cargo-xwin (MSVC)
cross-windows-xwin:
    ./scripts/build_windows_cross.sh --xwin --release

# Cross-compila para Windows usando MinGW-w64 GCC
cross-windows-mingw:
    ./scripts/build_windows_cross.sh --mingw --release

# Empacota versão Linux (.tar.gz) contendo executáveis e pasta assets
package-linux: build-release
    mkdir -p dist/guerra-fria
    cp target/release/guerra_fria dist/guerra-fria/
    cp target/release/gerador_mapa dist/guerra-fria/
    cp -r assets data dist/guerra-fria/
    test -f README.md && cp README.md dist/guerra-fria/ || true
    test -f BUILD.md && cp BUILD.md dist/guerra-fria/ || true
    tar -czvf guerra-fria-v{{versao}}-linux-x86_64.tar.gz -C dist/guerra-fria .
    @echo "Pacote Linux gerado com sucesso!"

# Empacota versão Windows (.zip) contendo executáveis e pasta assets
package-windows:
    ./scripts/build_windows_cross.sh --xwin --release

# Valida sintaxe e conformidade dos workflows do GitHub Actions
validate-ci:
    python3 scripts/validate_workflows.py

# Limpa diretórios temporários e caches de build
clean:
    cargo clean
