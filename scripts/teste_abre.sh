#!/usr/bin/env bash
set -euo pipefail

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$raiz"

cargo build -p guerra_fria

bin="${CARGO_TARGET_DIR:-target}/debug/guerra_fria"
log="$(mktemp)"
trap 'rm -f "$log"' EXIT

set +e
timeout 20 "$bin" >"$log" 2>&1
status=$?
set -e

if grep -qi "panicked" "$log"; then
    cat "$log" >&2
    echo "teste_abre: o jogo entrou em pânico" >&2
    exit 1
fi

if [[ "$status" -ne 0 && "$status" -ne 124 ]]; then
    cat "$log" >&2
    echo "teste_abre: o processo terminou com status $status" >&2
    exit "$status"
fi

echo "teste_abre: OK (processo observado por até 20 s)"
