# Promptus — helpers shared by the deploy/ scripts. Sourced, not run.
#
# Every script works from the repository root and drives the production
# stack through `compose`, which always passes the production file and
# its env file. PROMPTUS_ENV_FILE points at another env file (tests).

cd "$(dirname "${BASH_SOURCE[0]}")/.."

COMPOSE_FILE_PROD=docker-compose.prod.yml
ENV_FILE="${PROMPTUS_ENV_FILE:-.env.production}"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"; }
die() { echo "error: $*" >&2; exit 1; }

compose() {
  docker compose -f "$COMPOSE_FILE_PROD" --env-file "$ENV_FILE" "$@"
}

# Value of KEY in the env file, or of the KEY environment variable when
# set (the environment wins, as it does for compose), or $2.
setting() {
  local key="$1" default="${2:-}" value=""
  if [ -n "${!key:-}" ]; then
    printf '%s' "${!key}"
    return
  fi
  if [ -f "$ENV_FILE" ]; then
    value="$(awk -v k="$key" 'index($0, k "=") == 1 { print substr($0, length(k) + 2) }' "$ENV_FILE" | tail -n 1)"
  fi
  printf '%s' "${value:-$default}"
}

require_env_file() {
  [ -f "$ENV_FILE" ] || die "$ENV_FILE is missing: run deploy/install.sh first (docs/install.md)"
}

# psql inside the database container, as the owner, on the given
# database; stops at the first error.
psql_in() {
  local db="$1"; shift
  compose exec -T postgres psql -U promptus -d "$db" -v ON_ERROR_STOP=1 -qtA "$@"
}
