#!/usr/bin/env sh
set -eu

FROST_BIN=$1
SCRIPT=$2

HOST=${HTTPS_CA_TEST_HOST:-httpbin.org}
PORT=${HTTPS_CA_TEST_PORT:-443}

if ! command -v curl >/dev/null 2>&1; then
  echo "curl not available; skipping"
  exit 77
fi

if ! curl -fsS --connect-timeout 5 --max-time 10 "https://${HOST}:${PORT}/get" >/dev/null 2>&1; then
  echo "https ca-controls endpoint unavailable; skipping"
  exit 77
fi

ca_file=""
for candidate in \
  /etc/ssl/certs/ca-certificates.crt \
  /etc/ssl/cert.pem \
  /etc/pki/tls/certs/ca-bundle.crt \
  /etc/ssl/ca-bundle.pem; do
  if [ -f "$candidate" ]; then
    ca_file="$candidate"
    break
  fi
done

ca_path=""
for candidate in \
  /etc/ssl/certs \
  /etc/pki/tls/certs; do
  if [ -d "$candidate" ]; then
    ca_path="$candidate"
    break
  fi
done

if [ -z "$ca_file" ] || [ -z "$ca_path" ]; then
  echo "ca_file or ca_path not found; skipping"
  exit 77
fi

HTTPS_CA_TEST_HOST="$HOST" \
HTTPS_CA_TEST_PORT="$PORT" \
HTTPS_CA_TEST_FILE="$ca_file" \
HTTPS_CA_TEST_PATH="$ca_path" \
"$FROST_BIN" "$SCRIPT"
