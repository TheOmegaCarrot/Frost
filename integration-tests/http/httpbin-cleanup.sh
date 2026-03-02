#!/usr/bin/env sh
set -eu

case "$(printf '%s' "${FROST_SKIP_HTTP_TESTS:-}" | tr '[:lower:]' '[:upper:]')" in
  1|ON|TRUE|YES)
    exit 0
    ;;
esac

if ! command -v docker >/dev/null 2>&1; then
  exit 0
fi

NAME="frost-httpbin"

docker rm -f "$NAME" >/dev/null 2>&1 || true
