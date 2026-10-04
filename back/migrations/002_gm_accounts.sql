-- platform/sign-in-gm — game master accounts, passkeys, sessions and
-- invitations.
--
-- A GM has no password and no email: a passkey is the only way in, and
-- it is discoverable, so signing in names no account. Players never get
-- a row here (they join with a hashed device token, see
-- `session/invite-and-join`).
--
-- Every secret the server hands out (session cookie, invitation code)
-- is a 256-bit random token stored as its SHA-256: a database dump
-- yields nothing usable.

CREATE TABLE gms (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  display_name TEXT NOT NULL,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER gms_set_updated_at
  BEFORE UPDATE ON gms
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- A GM's passkeys. `passkey` is the serialised public credential
-- (webauthn-rs `Passkey`): a public key and its counter, nothing
-- secret. One authenticator credential belongs to one GM only.
CREATE TABLE gm_passkeys (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  gm_id         UUID NOT NULL REFERENCES gms(id) ON DELETE CASCADE,
  credential_id TEXT NOT NULL UNIQUE,
  passkey       JSONB NOT NULL,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_used_at  TIMESTAMPTZ
);

CREATE INDEX gm_passkeys_gm_idx ON gm_passkeys (gm_id);

-- Server-side sessions behind the `promptus_gm` cookie. Signing out
-- deletes the row; an expired row is refused and swept at the GM's next
-- sign-in.
CREATE TABLE gm_sessions (
  id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  gm_id      UUID NOT NULL REFERENCES gms(id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL UNIQUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX gm_sessions_gm_idx ON gm_sessions (gm_id);

-- An invitation for another GM to create an account: single use,
-- short-lived, minted by a signed-in GM who alone can revoke it.
CREATE TABLE gm_invites (
  id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  token_hash TEXT NOT NULL UNIQUE,
  created_by UUID NOT NULL REFERENCES gms(id) ON DELETE CASCADE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ NOT NULL,
  used_at    TIMESTAMPTZ,
  used_by    UUID REFERENCES gms(id) ON DELETE SET NULL
);

CREATE INDEX gm_invites_created_by_idx ON gm_invites (created_by);
