#!/usr/bin/env sh
set -eu

FROST_BIN=$1
SCRIPT=$2

PORT=${HTTPBIN_PORT:-8080}

if ! command -v docker >/dev/null 2>&1; then
  echo "docker not available; skipping"
  exit 77
fi

if ! docker info >/dev/null 2>&1; then
  echo "docker not running; skipping"
  exit 77
fi

if [ -z "$(docker ps -q -f "name=^/frost-httpbin$")" ]; then
  echo "httpbin container not running; skipping"
  exit 77
fi

HTTPBIN_HOST=127.0.0.1 HTTPBIN_PORT=$PORT "$FROST_BIN" "$SCRIPT"
