#!/bin/sh
# Select one of the three Caddyfiles by TLS_MODE (docs/09 §5) and run Caddy.
# /etc/caddy is mounted read-only, so the selected config is copied to /tmp.
set -eu
mode="${TLS_MODE:-auto}"
case "$mode" in
  auto|manual|internal) ;;
  *)
    echo "entrypoint: TLS_MODE must be auto|manual|internal, got '$mode'" >&2
    exit 1
    ;;
esac
config="/etc/caddy/Caddyfile.$mode"
if [ ! -f "$config" ]; then
  echo "entrypoint: missing $config" >&2
  exit 1
fi
if [ "$mode" = "manual" ]; then
  for file in fullchain.pem privkey.pem; do
    if [ ! -s "/etc/caddy/certs/$file" ]; then
      echo "entrypoint: TLS_MODE=manual requires /etc/caddy/certs/$file (deploy/caddy/certs/)" >&2
      exit 1
    fi
  done
fi
mkdir -p /tmp/caddy
cp "$config" /tmp/caddy/Caddyfile
echo "entrypoint: TLS_MODE=$mode DOMAIN=${DOMAIN:-<unset>} (caddy config: /tmp/caddy/Caddyfile)"
exec caddy run --config /tmp/caddy/Caddyfile --adapter caddyfile
