#!/usr/bin/env bash
# Promptus — restore a backup into the production database (docs/backup.md).
#
#   deploy/restore.sh <archive>            into an empty database
#   deploy/restore.sh <archive> --force    replace a database holding data
#
# An encrypted archive (.tar.age) needs the age private key:
# PROMPTUS_BACKUP_AGE_IDENTITY=/path/to/key.txt deploy/restore.sh <archive>
#
# Steps: check the archive, stop the app, (with --force: back up what is
# there first), recreate the database, pg_restore into it, start the
# app — which applies any migration newer than the backup — and wait for
# its health check.
set -euo pipefail
source "$(dirname "$0")/lib.sh"

archive="${1:-}"
force=0
[ "${2:-}" = "--force" ] && force=1
[ -n "$archive" ] || die "usage: deploy/restore.sh <archive> [--force]"
# lib.sh moved to the repository root: resolve a relative path from
# where the command was typed.
case "$archive" in /*) ;; *) archive="$OLDPWD/$archive" ;; esac
[ -f "$archive" ] || die "no such archive: $archive"
require_env_file

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

case "$archive" in
  *.age)
    identity="$(setting PROMPTUS_BACKUP_AGE_IDENTITY)"
    [ -n "$identity" ] || die "encrypted archive: set PROMPTUS_BACKUP_AGE_IDENTITY to the age private key file"
    command -v age > /dev/null || die "age is not installed"
    age -d -i "$identity" "$archive" | tar -x -C "$work"
    ;;
  *) tar -x -C "$work" -f "$archive" ;;
esac
[ -f "$work/db.dump" ] || die "$archive holds no db.dump"
log "Archive:"
sed 's/^/  /' "$work/manifest.txt" 2>/dev/null || true

log "Starting the database…"
compose up -d --wait postgres
# Read the dump's table of contents before touching anything.
compose exec -T postgres pg_restore --list < "$work/db.dump" > /dev/null \
  || die "db.dump is not a readable pg_dump archive"

# Rows in any table but the migration ledger: a freshly migrated, never
# used database counts as empty and is replaced without asking.
rows="$(psql_in postgres -c "SELECT count(*) FROM pg_database WHERE datname = 'promptus'")"
if [ "$rows" = 1 ]; then
  rows="$(psql_in promptus -c "
    SELECT coalesce(sum((xpath('/row/c/text()', query_to_xml(
             format('SELECT count(*) AS c FROM %I.%I', schemaname, relname),
             false, true, '')))[1]::text::bigint), 0)
    FROM pg_stat_user_tables WHERE relname <> '_sqlx_migrations'")"
fi
if [ "$rows" != 0 ]; then
  [ "$force" = 1 ] || die "the database holds $rows rows; add --force to replace it (a backup of it is taken first)"
  log "Backing up the current database before replacing it…"
  deploy/backup.sh
fi

log "Stopping the app…"
compose stop app

log "Recreating the database…"
psql_in postgres \
  -c "DROP DATABASE IF EXISTS promptus WITH (FORCE)" \
  -c "CREATE DATABASE promptus OWNER promptus"

log "Restoring…"
compose exec -T postgres pg_restore -U promptus -d promptus \
  --no-owner --exit-on-error --single-transaction < "$work/db.dump"

log "Starting the app (it applies newer migrations, then reports healthy)…"
compose up -d --wait app
log "Restore done."
