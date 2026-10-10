-- platform/sign-in-gm — a GM signs in with their email and a code sent
-- to it, instead of a passkey. Anyone may become a GM; what a GM owns
-- (campaigns) stays theirs alone.
--
-- Passkeys, the first-account setup code and GM invitations go away:
-- open sign-up needs no admission, and the app is only reachable over
-- the VPN (docs/install.md). An account created before this migration
-- has no email and can no longer sign in.

DROP TABLE gm_invites;
DROP TABLE gm_passkeys;

-- Stored lower-cased and trimmed (`auth::codes::normalize_email`).
ALTER TABLE gms ADD COLUMN email TEXT;
CREATE UNIQUE INDEX gms_email_idx ON gms (email);

-- At most one live code per address: asking again replaces it. The code
-- is six digits, kept as SHA-256 of `email:code`. Six digits are
-- enumerable from a dump, which is why a code lives ten minutes and
-- dies after five wrong tries.
CREATE TABLE gm_sign_in_codes (
  email      TEXT PRIMARY KEY,
  code_hash  TEXT NOT NULL,
  attempts   INT NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ NOT NULL
);
