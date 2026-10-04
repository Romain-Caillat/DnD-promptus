# Backup and restore

Production backups are taken by `deploy/backup.sh` and restored by
`deploy/restore.sh`, both run from the repository root on the machine
running the stack ([install.md](install.md)). The continuous deployment
takes one **before every deploy** and refuses to deploy when it fails.

## What an archive holds

`promptus-<YYYYmmdd>-<HHMMSS>.tar` — or `.tar.age` when encrypted — in
`PROMPTUS_BACKUP_DIR`:

| Entry | Content |
| --- | --- |
| `manifest.txt` | Date, deployed commit, last applied migration, PostgreSQL version |
| `db.dump` | `pg_dump --format=custom` of the `promptus` database |

The app stores no files yet; when media arrive (images, maps,
uploads), they join the archive and this page says so.

The dump runs inside the database container, so `pg_dump` always
matches the server's version. An archive is written under a temporary
name and renamed when complete: a crash never leaves a truncated archive
that looks valid. Archives are mode 600.

## Settings (`.env.production`)

| Variable | Default | Meaning |
| --- | --- | --- |
| `PROMPTUS_BACKUP_DIR` | `./backups` | Where archives go, relative to the repository root. **Put it on another disk than the database** (on Proxmox: a mount point onto the host's ZFS pool) — a backup on the disk that dies does not help |
| `PROMPTUS_BACKUP_KEEP` | `30` | Archives kept; older ones are deleted at each backup |
| `PROMPTUS_BACKUP_AGE_RECIPIENT` | empty | An [age](https://age-encryption.org) public key (`age1…`). When set, archives are encrypted to it. Needs `age` on the machine |

Each setting can also be given in the environment for one run, which
wins over the file.

### Encryption

Without a recipient, archives are plain tar files readable by root on
the machine — fine while they stay on the server's own disks. Before
copying them anywhere else, turn encryption on:

```bash
age-keygen -o promptus-backup-key.txt   # on YOUR machine, not the server
# put the printed public key (age1…) in PROMPTUS_BACKUP_AGE_RECIPIENT
```

The server only holds the public key: a stolen archive or server
cannot decrypt the backups. ⚠️ **Losing the private key makes every
encrypted archive unreadable.** Keep it in a password manager.

## Back up now

```bash
deploy/backup.sh      # prints the archive path on its last line
```

## Restore

```bash
deploy/restore.sh backups/promptus-20261004-103054.tar
```

What it does, in order:

1. Unpacks the archive and checks `db.dump` is a readable dump, before
   touching anything.
2. **Refuses a database that holds data**, unless `--force` is given. A
   database with only the migration ledger (a fresh install) counts as
   empty. With `--force`, it first runs `deploy/backup.sh`, so the
   replaced data is never lost.
3. Stops the app, drops and recreates the `promptus` database,
   `pg_restore`s into it in a single transaction (all or nothing).
4. Starts the app, which applies any migration newer than the backup,
   and waits for its health check.

An encrypted archive needs the private key:

```bash
PROMPTUS_BACKUP_AGE_IDENTITY=/path/to/promptus-backup-key.txt \
  deploy/restore.sh backups/promptus-<date>-<time>.tar.age
```

**A backup newer than the code** (taken after a deploy you rolled back)
holds migrations the code does not know: the server refuses to start.
Check out the commit in the archive's `manifest.txt` first.

### Restore on a brand-new machine

Clone the repository, copy the old `.env.production` (or run
`deploy/install.sh --no-start` for a new one), then:

```bash
deploy/restore.sh /path/to/promptus-<date>-<time>.tar
```

It starts the database, restores into it, then builds and starts the
app.

## Opening an archive without Promptus

```bash
tar -xf promptus-<date>-<time>.tar                    # or: age -d -i key.txt <file>.tar.age | tar -x
pg_restore --list db.dump                             # what is inside
pg_restore -d <some database> --no-owner db.dump      # client 17 or newer
```

## Verified

On 2026-10-04, in PCT 105, with a separate test stack: a table with
two rows was backed up, the database volume deleted (`down -v`), a
fresh stack started (migrations only), the archive restored without
`--force`, the rows were back and `/api/health` answered 200. Same with
an encrypted archive, with `--force` refusal and rotation.
