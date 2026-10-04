#!/usr/bin/env bash
# Promptus — back up the production database (docs/backup.md).
#
#   deploy/backup.sh            write one archive, print its path last
#
# The archive is promptus-<date>-<time>.tar (or .tar.age when
# PROMPTUS_BACKUP_AGE_RECIPIENT is set) in PROMPTUS_BACKUP_DIR, holding:
#   manifest.txt   when, which commit, which migration
#   db.dump        pg_dump custom format, restorable with pg_restore
# Media files will join it when the app stores some.
#
# The dump runs inside the database container, so pg_dump always matches
# the server's version. Archives are written under a temporary name and
# renamed at the end: a crash never leaves a truncated one that looks
# valid. Exits non-zero on any failure — deploy.sh relies on that (no
# backup, no deploy).
set -euo pipefail
source "$(dirname "$0")/lib.sh"

require_env_file
dir="$(setting PROMPTUS_BACKUP_DIR ./backups)"
keep="$(setting PROMPTUS_BACKUP_KEEP 30)"
recipient="$(setting PROMPTUS_BACKUP_AGE_RECIPIENT)"
case "$keep" in ''|*[!0-9]*|0) die "PROMPTUS_BACKUP_KEEP must be a positive number, got: $keep" ;; esac
if [ -n "$recipient" ]; then
  command -v age > /dev/null || die "PROMPTUS_BACKUP_AGE_RECIPIENT is set but age is not installed"
fi

compose exec -T postgres pg_isready -h 127.0.0.1 -U promptus -d promptus > /dev/null \
  || die "the database container is not running (compose ps)"

mkdir -p "$dir"
umask 077
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

stamp="$(date +%Y%m%d-%H%M%S)"
name="promptus-$stamp.tar"
[ -n "$recipient" ] && name="$name.age"

log "Dumping the database…"
compose exec -T postgres pg_dump -U promptus -d promptus --format=custom > "$work/db.dump"
# A custom-format dump starts with PGDMP; anything else is an error page.
[ "$(head -c 5 "$work/db.dump")" = PGDMP ] || die "pg_dump produced no valid dump"

{
  echo "created: $(date -Iseconds)"
  echo "commit: $(git rev-parse HEAD 2>/dev/null || echo unknown)"
  echo "migration: $(psql_in promptus -c 'SELECT max(version) FROM _sqlx_migrations' 2>/dev/null || echo none)"
  echo "postgres: $(psql_in promptus -c 'SHOW server_version')"
} > "$work/manifest.txt"

tar -C "$work" -cf "$work/archive.tar" manifest.txt db.dump
if [ -n "$recipient" ]; then
  age -r "$recipient" -o "$work/archive.tar.age" "$work/archive.tar"
  mv "$work/archive.tar.age" "$dir/.$name.partial"
else
  mv "$work/archive.tar" "$dir/.$name.partial"
fi
mv "$dir/.$name.partial" "$dir/$name"

# Rotation: keep the newest $keep archives (names sort by date).
find "$dir" -maxdepth 1 -type f -name 'promptus-*.tar*' -printf '%f\n' \
  | sort -r | tail -n +"$((keep + 1))" \
  | while read -r old; do rm -f "$dir/$old"; done

log "Backup written ($(du -h --apparent-size "$dir/$name" | cut -f1)):"
echo "$dir/$name"
