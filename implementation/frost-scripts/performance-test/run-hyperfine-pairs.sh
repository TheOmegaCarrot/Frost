#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

FROST_BIN="${1:-${FROST_BIN:-${REPO_ROOT}/rel/frost}}"
PYTHON_BIN="${PYTHON_BIN:-python3}"
HYPERFINE_BIN="${HYPERFINE_BIN:-hyperfine}"
WARMUP="${WARMUP:-3}"
RUNS="${RUNS:-20}"

if ! command -v "${HYPERFINE_BIN}" >/dev/null 2>&1; then
    echo "error: hyperfine not found: ${HYPERFINE_BIN}" >&2
    exit 1
fi

if ! command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
    echo "error: python not found: ${PYTHON_BIN}" >&2
    exit 1
fi

if [[ ! -x "${FROST_BIN}" ]]; then
    echo "error: frost binary is not executable: ${FROST_BIN}" >&2
    exit 1
fi

for frost_script in "${SCRIPT_DIR}"/*.frst; do
    base_name="$(basename "${frost_script}" .frst)"
    python_script="${SCRIPT_DIR}/${base_name}.py"

    if [[ ! -f "${python_script}" ]]; then
        echo "error: missing python pair for ${frost_script}" >&2
        exit 1
    fi

    frost_cmd="\"${FROST_BIN}\" \"${frost_script}\""
    python_cmd="\"${PYTHON_BIN}\" \"${python_script}\""

    echo "=== ${base_name} ==="
    "${HYPERFINE_BIN}" \
        --shell=none \
        --warmup "${WARMUP}" \
        --runs "${RUNS}" \
        -n "frost:${base_name}" "${frost_cmd}" \
        -n "python:${base_name}" "${python_cmd}"

    printf "\n\n\n\n"
done
