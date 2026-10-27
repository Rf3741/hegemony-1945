#!/usr/bin/env bash
# ==============================================================================
# Script de Cross-Compilação para Windows a partir do Linux
# Projeto: Guerra Fria (1945+) - Rust + Bevy
# ==============================================================================
#
# Este script suporta DUAS abordagens para gerar binários de Windows (.exe):
#
# ------------------------------------------------------------------------------
# OPÇÃO 1: cargo-xwin (Target: x86_64-pc-windows-msvc) - RECOMENDADO
# ------------------------------------------------------------------------------
# A ferramenta `cargo-xwin` utiliza o compilador LLVM/Clang integrado ao Rust e
# baixa os cabeçalhos oficiais do Windows SDK e CRT da Microsoft sem precisar de
# uma máquina Windows ou Wine.
#
# Vantagens:
#   - Compatibilidade total com a ABI oficial do Windows (MSVC).
#   - Funciona perfeitamente com bibliotecas nativas C/C++ e shaders do Bevy/wgpu.
#   - Gera símbolos de depuração PDB compatíveis com ferramentas Windows.
#
# Pré-requisitos para Opção 1:
#   1. Instalar cargo-xwin:
#      cargo install --locked cargo-xwin
#   2. Adicionar o target Rustup:
#      rustup target add x86_64-pc-windows-msvc
#
# ------------------------------------------------------------------------------
# OPÇÃO 2: MinGW-w64 (Target: x86_64-pc-windows-gnu)
# ------------------------------------------------------------------------------
# Utiliza a toolchain GNU MinGW disponível diretamente nos repositórios Linux.
#
# Vantagens:
#   - 100% open-source e empacotado nas principais distribuições Linux.
#   - Fácil de instalar via apt ou pacman sem downloads externos.
#
# Pré-requisitos para Opção 2:
#   1. Instalar o compilador MinGW-w64 do sistema:
#      Ubuntu / Debian: sudo apt-get update && sudo apt-get install -y mingw-w64
#      Arch Linux:      sudo pacman -S mingw-w64-gcc
#      Fedora:          sudo dnf install -y mingw64-gcc
#   2. Adicionar o target Rustup:
#      rustup target add x86_64-pc-windows-gnu
#   3. O arquivo .cargo/config.toml já está configurado para apontar para:
#      linker = "x86_64-w64-mingw32-gcc"
#      ar = "x86_64-w64-mingw32-ar"
#
# ==============================================================================

set -euo pipefail

# Diretório raiz do projeto (onde está o Cargo.toml do workspace)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Configurações padrão
TOOL="xwin"
PROFILE="release"
DO_PACKAGE=true
BIN_NAME="guerra_fria"
MAP_BIN_NAME="gerador_mapa"
VERSION="${VERSION:-$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "${WORKSPACE_ROOT}/Cargo.toml")}"

exibir_ajuda() {
    cat << EOF
Uso: $0 [OPÇÕES]

Opções:
  --xwin          Usa cargo-xwin (target: x86_64-pc-windows-msvc) [Padrão]
  --mingw         Usa MinGW GCC (target: x86_64-pc-windows-gnu)
  --release       Compila em modo release otimizado [Padrão]
  --debug         Compila em modo debug (mais rápido para testar)
  --no-package    Apenas compila, não gera o arquivo .zip final
  --help, -h      Exibe este texto de ajuda

Exemplos:
  $0 --xwin --release
  $0 --mingw --debug
EOF
}

# Parse de argumentos
while [[ $# -gt 0 ]]; do
    case "$1" in
        --xwin)
            TOOL="xwin"
            shift
            ;;
        --mingw)
            TOOL="mingw"
            shift
            ;;
        --release)
            PROFILE="release"
            shift
            ;;
        --debug)
            PROFILE="debug"
            shift
            ;;
        --no-package)
            DO_PACKAGE=false
            shift
            ;;
        --help|-h)
            exibir_ajuda
            exit 0
            ;;
        *)
            echo "Erro: Opção desconhecida: $1"
            exibir_ajuda
            exit 1
            ;;
    esac
done

cd "${WORKSPACE_ROOT}"

echo "======================================================================"
echo " Início do Cross-Compile para Windows"
echo " Ferramenta: ${TOOL}"
echo " Perfil:     ${PROFILE}"
echo " Workspace:  ${WORKSPACE_ROOT}"
echo "======================================================================"

BUILD_FLAGS=()
if [[ "${PROFILE}" == "release" ]]; then
    BUILD_FLAGS+=(--release)
fi

if [[ "${TOOL}" == "xwin" ]]; then
    TARGET="x86_64-pc-windows-msvc"
    echo ">> Validando dependências do cargo-xwin..."

    if ! command -v cargo-xwin >/dev/null 2>&1; then
        echo "ERRO: 'cargo-xwin' não está instalado."
        echo "Instale com:"
        echo "  cargo install --locked cargo-xwin"
        echo "  rustup target add ${TARGET}"
        exit 1
    fi

    echo ">> Compilando via cargo xwin (target: ${TARGET})..."
    cargo xwin build --target "${TARGET}" "${BUILD_FLAGS[@]}" -p guerra_fria -p gerador_mapa

elif [[ "${TOOL}" == "mingw" ]]; then
    TARGET="x86_64-pc-windows-gnu"
    echo ">> Validando dependências do MinGW-w64..."

    if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
        echo "ERRO: O compilador 'x86_64-w64-mingw32-gcc' não foi encontrado."
        echo "Instale com:"
        echo "  Ubuntu/Debian: sudo apt-get install -y mingw-w64"
        echo "  Arch Linux:    sudo pacman -S mingw-w64-gcc"
        echo "  rustup target add ${TARGET}"
        exit 1
    fi

    echo ">> Compilando via cargo + MinGW (target: ${TARGET})..."
    cargo build --target "${TARGET}" "${BUILD_FLAGS[@]}" -p guerra_fria -p gerador_mapa
fi

OUTPUT_DIR="${WORKSPACE_ROOT}/target/${TARGET}/${PROFILE}"
echo ">> Compilação concluída com sucesso em: ${OUTPUT_DIR}"

if [[ "${DO_PACKAGE}" == "true" ]]; then
    ZIP_NAME="guerra-fria-v${VERSION}-windows-x86_64.zip"
    STAGING_DIR="${WORKSPACE_ROOT}/dist/package_windows"

    echo ">> Empacotando distribuição para Windows..."
    mkdir -p "${STAGING_DIR}"

    # Copia os executáveis compilados
    if [[ -f "${OUTPUT_DIR}/${BIN_NAME}.exe" ]]; then
        cp "${OUTPUT_DIR}/${BIN_NAME}.exe" "${STAGING_DIR}/"
    fi
    if [[ -f "${OUTPUT_DIR}/${MAP_BIN_NAME}.exe" ]]; then
        cp "${OUTPUT_DIR}/${MAP_BIN_NAME}.exe" "${STAGING_DIR}/"
    fi

    # Copia a pasta de assets
    for d in assets data; do
        if [[ -d "${WORKSPACE_ROOT}/${d}" ]]; then
            cp -r "${WORKSPACE_ROOT}/${d}" "${STAGING_DIR}/"
        fi
    done

    # Copia documentação
    if [[ -f "${WORKSPACE_ROOT}/README.md" ]]; then
        cp "${WORKSPACE_ROOT}/README.md" "${STAGING_DIR}/"
    fi
    if [[ -f "${WORKSPACE_ROOT}/BUILD.md" ]]; then
        cp "${WORKSPACE_ROOT}/BUILD.md" "${STAGING_DIR}/"
    fi

    # Gera o arquivo zip se zip ou python estiver disponível
    if command -v zip >/dev/null 2>&1; then
        (cd "${STAGING_DIR}" && zip -r "${WORKSPACE_ROOT}/${ZIP_NAME}" ./*)
    else
        python3 -c "
import shutil, os
src = '${STAGING_DIR}'
dest = '${WORKSPACE_ROOT}/guerra-fria-v${VERSION}-windows-x86_64'
shutil.make_archive(dest, 'zip', src)
"
    fi

    echo ">> Pacote gerado com sucesso: ${WORKSPACE_ROOT}/${ZIP_NAME}"
fi

echo "======================================================================"
echo " Concluído com sucesso!"
echo "======================================================================"
