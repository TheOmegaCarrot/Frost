#!/usr/bin/env sh
set -eu

case "$(printf '%s' "${FROST_SKIP_HTTP_TESTS:-}" | tr '[:lower:]' '[:upper:]')" in
  1|ON|TRUE|YES)
    echo "FROST_SKIP_HTTP_TESTS set; skipping"
    exit 77
    ;;
esac

if ! command -v docker >/dev/null 2>&1; then
  echo "docker not available; skipping"
  exit 77
fi

if ! docker info >/dev/null 2>&1; then
  echo "docker not running; skipping"
  exit 77
fi

PORT=${HTTPBIN_PORT:-8080}
NAME="frost-httpbin"
IMAGE="kennethreitz/httpbin"

running_id=$(docker ps -q -f "name=^/${NAME}$")
if [ -n "$running_id" ]; then
  exit 0
fi

existing_id=$(docker ps -aq -f "name=^/${NAME}$")
if [ -n "$existing_id" ]; then
  docker rm -f "$NAME" >/dev/null 2>&1 || true
fi

docker run -d --rm --name "$NAME" -p "127.0.0.1:${PORT}:80" "$IMAGE" >/dev/null

if command -v curl >/dev/null 2>&1; then
  i=0
  while [ $i -lt 20 ]; do
    if curl -fsS "http://127.0.0.1:${PORT}/get" >/dev/null 2>&1; then
      exit 0
    fi
    i=$((i + 1))
    sleep 0.2
  done
  echo "httpbin failed to start"
  exit 1
fi

sleep 1
