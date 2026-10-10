-- platform/connect-claude-mcp — a GM's personal access tokens.
--
-- A token lets a program the GM runs on their own machine (the local
-- MCP server Claude launches) call a short list of preparation routes
-- with `Authorization: Bearer <token>` (`auth::api_tokens::TOKEN_ROUTES`).
-- Like every secret the server hands out, it is a 256-bit random token
-- stored as its SHA-256, shown once at creation. Revoking keeps the row
-- (when, by whom it was last used) but the token stops working at once.
CREATE TABLE gm_api_tokens (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  gm_id        UUID NOT NULL REFERENCES gms(id) ON DELETE CASCADE,
  name         TEXT NOT NULL,
  token_hash   TEXT NOT NULL UNIQUE,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_used_at TIMESTAMPTZ,
  revoked_at   TIMESTAMPTZ
);

CREATE INDEX gm_api_tokens_gm_idx ON gm_api_tokens (gm_id) WHERE revoked_at IS NULL;
