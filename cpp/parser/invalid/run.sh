#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
REL_CASES="${1:-cases}"

if [[ "$REL_CASES" = /* ]]; then
  CASES_DIR="$REL_CASES"
elif [[ -d "$SCRIPT_DIR/$REL_CASES" ]]; then
  CASES_DIR="$SCRIPT_DIR/$REL_CASES"
elif [[ -d "$ROOT_DIR/$REL_CASES" ]]; then
  CASES_DIR="$ROOT_DIR/$REL_CASES"
else
  echo "Cases directory not found: $REL_CASES" >&2
  exit 1
fi

OUT_DIR="$SCRIPT_DIR/outputs"

mkdir -p "$OUT_DIR"

shopt -s nullglob
files=("$CASES_DIR"/*.frst)

if (( ${#files[@]} == 0 )); then
  echo "No .frst files found in $CASES_DIR" >&2
  exit 1
fi

for file in "${files[@]}"; do
  base="$(basename "$file" .frst)"
  out="$OUT_DIR/${base}.txt"
  if ! "${ROOT_DIR}/build/frost" "$file" 2>&1 | tee "$out"; then
    : # Expected for invalid programs.
  fi
  echo "Wrote $out"
done
