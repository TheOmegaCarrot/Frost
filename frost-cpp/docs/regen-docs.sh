#!/bin/bash

# Resolve paths relative to this script, not CWD
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# My own convention: debug build in build, release build in rel
if [[ -z "${FROST:-}" ]]; then
    if [[ -x "$ROOT_DIR/rel/frost" ]]; then
        FROST="$ROOT_DIR/rel/frost"
    elif [[ -x "$ROOT_DIR/build/frost" ]]; then
        FROST="$ROOT_DIR/build/frost"
    else
        echo "Frost executable not found" >&2
        exit 1
    fi
fi

if [[ -z "${FROST_HIGHLIGHT:-}" ]]; then
    if [[ -x "$ROOT_DIR/rel/frost-highlight" ]]; then
        FROST_HIGHLIGHT="$ROOT_DIR/rel/frost-highlight"
    elif [[ -x "$ROOT_DIR/build/frost-highlight" ]]; then
        FROST_HIGHLIGHT="$ROOT_DIR/build/frost-highlight"
    else
        echo "frost-highlight not found (build with -DBUILD_HIGHLIGHT=YES)" >&2
        exit 1
    fi
fi

export FROST_HIGHLIGHT

"$FROST" "$SCRIPT_DIR/src/generator.frst" --format markdown --output "$SCRIPT_DIR/md/"
"$FROST" "$SCRIPT_DIR/src/generator.frst" --format html --output "$SCRIPT_DIR/"
