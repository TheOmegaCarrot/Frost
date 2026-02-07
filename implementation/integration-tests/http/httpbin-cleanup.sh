#!/usr/bin/env sh
set -eu

if ! command -v docker >/dev/null 2>&1; then
  exit 0
fi

NAME="frost-httpbin"

docker rm -f "$NAME" >/dev/null 2>&1 || true
