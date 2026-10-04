#!/bin/bash
# Promptus — deploy the latest commit on main to production
# (docs/install.md, "Continuous deployment"). Modelled on Devotion's.
#
# Runs on the Proxmox host from cron, every 5 minutes. The host holds the
# GitHub key and the container running the stack does not, so the code
# travels as a git bundle. In the container: backup first (no backup, no
# deploy), then rebuild and restart, waiting for the health check.
# Rolling back is manual: docs/install.md, "Roll back".
#
# Install, once the target container is chosen and holds a checkout
# with its .env.production (docs/install.md):
#   install -m 0755 deploy/prod-deploy.sh /usr/local/sbin/promptus-deploy
#   echo 'PROMPTUS_CTID=<ctid>' > /etc/default/promptus-deploy
# and this crontab line:
#   */5 * * * * /usr/local/sbin/promptus-deploy >> /var/log/promptus-deploy.log 2>&1
set -euo pipefail

# shellcheck disable=SC1091
[ -f /etc/default/promptus-deploy ] && . /etc/default/promptus-deploy
CTID="${PROMPTUS_CTID:?set PROMPTUS_CTID in /etc/default/promptus-deploy}"
PROD_DIR="${PROMPTUS_PROD_DIR:-/opt/promptus}"
REMOTE="${PROMPTUS_REMOTE:-git@github.com:Romain-Caillat/DnD-promptus.git}"
BRANCH="${PROMPTUS_BRANCH:-main}"
STATE=/var/lib/promptus-deploy
MIRROR=$STATE/mirror.git
# A commit that failed to deploy is not retried: each try takes a backup,
# and a loop of them would rotate the good ones out (PROMPTUS_BACKUP_KEEP).
# The next commit on main is tried. To retry the same one: rm this file.
FAILED=$STATE/failed

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"; }

exec 9> /run/promptus-deploy.lock
flock -n 9 || exit 0

mkdir -p "$STATE"
[ -d "$MIRROR" ] || git init --quiet --bare "$MIRROR"
git -C "$MIRROR" fetch --quiet "$REMOTE" "+refs/heads/$BRANCH:refs/heads/$BRANCH"

CURRENT=$(/usr/sbin/pct exec "$CTID" -- git -C "$PROD_DIR" rev-parse HEAD)
TARGET=$(git -C "$MIRROR" rev-parse "refs/heads/$BRANCH")
[ "$CURRENT" = "$TARGET" ] && exit 0
[ "$TARGET" = "$(cat "$FAILED" 2>/dev/null)" ] && exit 0

log "New commit: $CURRENT -> $TARGET"

git -C "$MIRROR" bundle create --quiet /tmp/promptus.bundle "refs/heads/$BRANCH"
/usr/sbin/pct push "$CTID" /tmp/promptus.bundle /root/promptus.bundle
rm -f /tmp/promptus.bundle

echo "$TARGET" > "$FAILED"
if ! /usr/sbin/pct exec "$CTID" -- bash -euo pipefail -c "
  cd $PROD_DIR
  echo 'Backing up...'
  deploy/backup.sh
  git fetch --quiet /root/promptus.bundle $BRANCH
  git checkout --quiet --detach $TARGET
  echo 'Rebuilding and restarting...'
  docker compose -f docker-compose.prod.yml --env-file .env.production up -d --build --wait
  docker image prune -f > /dev/null
  docker builder prune -f --filter until=72h > /dev/null
"; then
  log "Deploy FAILED: $TARGET (not retried; see above)"
  exit 1
fi

rm -f "$FAILED"
log "Deploy done: $TARGET"
