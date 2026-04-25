#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

FROST_BIN="${1:-${FROST_BIN:-${REPO_ROOT}/rel/frost}}"
PYTHON_BIN="${PYTHON_BIN:-python3}"
RUBY_BIN="${RUBY_BIN:-ruby}"
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

HAVE_RUBY=false
if command -v "${RUBY_BIN}" >/dev/null 2>&1; then
    HAVE_RUBY=true
fi

for frost_script in "${SCRIPT_DIR}"/*.frst; do
    base_name="$(basename "${frost_script}" .frst)"
    python_script="${SCRIPT_DIR}/${base_name}.py"

    if [[ ! -f "${python_script}" ]]; then
        echo "error: missing python pair for ${frost_script}" >&2
        exit 1
    fi

    ruby_script="${SCRIPT_DIR}/${base_name}.rb"

    benchmarks=(
        -n "frost:${base_name}" "\"${FROST_BIN}\" \"${frost_script}\""
        -n "python:${base_name}" "\"${PYTHON_BIN}\" \"${python_script}\""
    )

    if [[ "${HAVE_RUBY}" == true && -f "${ruby_script}" ]]; then
        benchmarks+=(
            -n "ruby:${base_name}" "\"${RUBY_BIN}\" \"${ruby_script}\""
        )
    fi

    echo "=== ${base_name} ==="
    "${HYPERFINE_BIN}" \
        --shell=none \
        --warmup "${WARMUP}" \
        --runs "${RUNS}" \
        "${benchmarks[@]}"

    printf "\n\n\n\n"
done
