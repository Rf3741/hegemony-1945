# ==============================================================================
# Makefile - Guerra Fria (1945+)
# Workspace Cargo: guerra_fria (jogo), gerador_mapa (CLI), simulacao + sim_* + bandeiras_resolver (libs)
# ==============================================================================

.DEFAULT_GOAL := help
SHELL := /usr/bin/env bash

# Variáveis
CARGO ?= cargo
PYTHON ?= python3
VERSION := $(shell sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml)
TARGET_LINUX := x86_64-unknown-linux-gnu
TARGET_WIN_MSVC := x86_64-pc-windows-msvc
TARGET_WIN_GNU := x86_64-pc-windows-gnu

.PHONY: help check fmt fmt-check clippy test build build-release build-fast \
        run run-fast run-map-gen cross-windows-xwin cross-windows-mingw \
        package-linux package-windows validate-ci clean

help: ## Exibe esta lista de comandos disponíveis
	@echo "Comandos disponíveis no projeto Guerra Fria:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2}'

check: ## Executa checagem de tipos e sintaxe em todo o workspace
	$(CARGO) check --workspace --all-targets

fmt: ## Formata todo o código-fonte com rustfmt
	$(CARGO) fmt --all

fmt-check: ## Verifica se o código-fonte está de acordo com as regras de formatação
	$(CARGO) fmt --all -- --check

clippy: ## Executa o linter Clippy em todo o workspace tratando warnings como erros
	$(CARGO) clippy --workspace --all-targets -- -D warnings

test: ## Executa todos os testes unitários e de integração
	$(CARGO) test --workspace --exclude guerra_fria --verbose

build: ## Compila o workspace em modo debug (desenvolvimento)
	$(CARGO) build --workspace

build-release: ## Compila o workspace em modo release (com LTO e otimizações)
	$(CARGO) build --workspace --release

build-fast: ## Compila o jogo principal com suporte a dynamic_linking (mais rápido)
	$(CARGO) build --package guerra_fria --features dynamic_linking

run: ## Executa o jogo principal Guerra Fria
	$(CARGO) run --package guerra_fria

run-fast: ## Executa o jogo principal com dynamic_linking para iteração rápida
	$(CARGO) run --package guerra_fria --features dynamic_linking

run-map-gen: ## Executa a ferramenta de geração do mapa
	$(CARGO) run --package gerador_mapa

cross-windows-xwin: ## Cross-compila para Windows via cargo-xwin (MSVC)
	./scripts/build_windows_cross.sh --xwin --release

cross-windows-mingw: ## Cross-compila para Windows via MinGW GCC
	./scripts/build_windows_cross.sh --mingw --release

package-linux: build-release ## Gera pacote de distribuição Linux (.tar.gz com assets)
	@mkdir -p dist/guerra-fria
	@cp target/release/guerra_fria dist/guerra-fria/
	@cp target/release/gerador_mapa dist/guerra-fria/
	@cp -r assets data dist/guerra-fria/
	@test -f README.md && cp README.md dist/guerra-fria/ || true
	@test -f BUILD.md && cp BUILD.md dist/guerra-fria/ || true
	tar -czvf "guerra-fria-v$(VERSION)-linux-x86_64.tar.gz" -C dist/guerra-fria .
	@echo "Pacote Linux gerado: guerra-fria-v$(VERSION)-linux-x86_64.tar.gz"

package-windows: ## Gera pacote de distribuição Windows (.zip com assets via cross-compile)
	./scripts/build_windows_cross.sh --xwin --release

validate-ci: ## Valida a sintaxe e conformidade dos workflows do GitHub Actions
	$(PYTHON) scripts/validate_workflows.py

clean: ## Limpa artefatos temporários de build e pacotes gerados
	$(CARGO) clean
	rm -rf dist *.tar.gz *.zip
