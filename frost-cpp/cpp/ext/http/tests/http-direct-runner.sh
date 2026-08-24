#!/usr/bin/env sh
set -eu

FROST_BIN=$1
SCRIPT=$2

case "$(printf '%s' "${FROST_SKIP_HTTP_TESTS:-}" | tr '[:lower:]' '[:upper:]')" in
  1|ON|TRUE|YES)
    echo "FROST_SKIP_HTTP_TESTS set; skipping"
    exit 77
    ;;
esac

"$FROST_BIN" "$SCRIPT"
