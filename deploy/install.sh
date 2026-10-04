#!/usr/bin/env bash
# Promptus — first install on the machine that runs the stack
# (docs/install.md).
#
#   deploy/install.sh              ask the public name, write .env.production, start
#   deploy/install.sh --no-start   only write .env.production
#
# Non-interactive: set PROMPTUS_DOMAIN in the environment beforehand.
# An existing .env.production is never overwritten: re-running the
# script only builds and starts the stack.
set -euo pipefail
source "$(dirname "$0")/lib.sh"

start=1
[ "${1:-}" = "--no-start" ] && start=0

command -v openssl > /dev/null || die "openssl is missing"
if [ "$start" = 1 ]; then
  docker compose version > /dev/null 2>&1 || die "Docker with the compose plugin is missing"
fi

# Replace KEY=... in the env file, whatever the value contains.
set_var() {
  local tmp
  tmp="$(mktemp)"
  awk -v k="$1" -v v="$2" 'BEGIN { done = 0 }
    index($0, k "=") == 1 { print k "=" v; done = 1; next }
    { print }
    END { if (!done) print k "=" v }' "$ENV_FILE" > "$tmp"
  cat "$tmp" > "$ENV_FILE"
  rm -f "$tmp"
}

if [ -f "$ENV_FILE" ]; then
  log "Keeping the existing $ENV_FILE."
else
  domain="${PROMPTUS_DOMAIN:-}"
  if [ -z "$domain" ]; then
    echo "Public name players will type, without https:// (promptus.example.org)."
    read -r -p "Name: " domain
  fi
  domain="${domain#https://}"; domain="${domain#http://}"; domain="${domain%/}"
  [ -n "$domain" ] || die "a name is required"

  umask 077
  cp .env.production.example "$ENV_FILE"
  chmod 600 "$ENV_FILE"
  set_var PROMPTUS_DOMAIN "$domain"
  set_var POSTGRES_PASSWORD "$(openssl rand -hex 24)"
  log "Wrote $ENV_FILE with a fresh database password."
fi

mkdir -p "$(setting PROMPTUS_BACKUP_DIR ./backups)"
[ "$start" = 0 ] && exit 0

log "Building and starting Promptus (the first build takes several minutes)…"
compose up -d --build --wait

bind="$(setting PROMPTUS_BIND 127.0.0.1)"
port="$(setting PROMPTUS_PORT 4380)"
curl -fsS "http://$bind:$port/api/health" > /dev/null \
  && log "Promptus answers on http://$bind:$port — now route https://$(setting PROMPTUS_DOMAIN) to it (docs/install.md)."
