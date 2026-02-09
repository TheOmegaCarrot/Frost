#!/usr/bin/env sh
set -eu

FROST_BIN=$1
SCRIPT=$2

HOST=${HTTPS_VERIFY_TEST_HOST:-self-signed.badssl.com}
PORT=${HTTPS_VERIFY_TEST_PORT:-443}

if ! command -v curl >/dev/null 2>&1; then
  echo "curl not available; skipping"
  exit 77
fi

# Use -k only for reachability preflight.
if ! curl -kfsS --connect-timeout 5 --max-time 10 "https://${HOST}:${PORT}/" >/dev/null 2>&1; then
  echo "https verify-toggle endpoint unavailable; skipping"
  exit 77
fi

HTTPS_VERIFY_TEST_HOST="$HOST" HTTPS_VERIFY_TEST_PORT="$PORT" "$FROST_BIN" "$SCRIPT"
