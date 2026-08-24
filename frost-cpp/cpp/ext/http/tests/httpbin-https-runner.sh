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

HOST=${HTTPBIN_TLS_HOST:-httpbin.org}
PORT=${HTTPBIN_TLS_PORT:-443}

if ! command -v curl >/dev/null 2>&1; then
  echo "curl not available; skipping"
  exit 77
fi

if ! curl -fsS --connect-timeout 5 --max-time 10 \
    "https://${HOST}:${PORT}/get" >/dev/null 2>&1; then
  echo "https httpbin unavailable; skipping"
  exit 77
fi

HTTPBIN_TLS_HOST="$HOST" HTTPBIN_TLS_PORT="$PORT" "$FROST_BIN" "$SCRIPT"
