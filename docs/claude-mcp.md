# Claude MCP access (`platform/connect-claude-mcp`)

The GM prepares a campaign by talking to their own Claude (Claude
Desktop or Claude Code on their computer). Claude launches a small
local MCP server (`mcp/`, stdio) that calls the Promptus API with a
**GM personal access token**. Claude on the web is out of scope: it
would need a server reachable from the Internet with OAuth.

```
Claude Desktop / Claude Code ──stdio──> mcp/ (bun) ──HTTPS + Bearer──> Promptus /api
```

## Tokens

- Minted, named, listed and revoked from the GM home (« Accès pour
  Claude »), through `GET/POST /api/gm-tokens` and
  `DELETE /api/gm-tokens/{id}`. These routes take the GM's session
  cookie only (`SessionOnly`): a token never mints or revokes a token.
- The secret is `promptus_` + 32 random bytes (base64url), shown once
  in the `POST` answer. The table `gm_api_tokens` stores its SHA-256;
  the lookup is a unique-index hit on the hash, so nothing compares the
  secret byte by byte. Each accepted call stamps `last_used_at`.
- Revoking sets `revoked_at`: the next call answers 401
  `INVALID_TOKEN`. A GM holds at most 20 live tokens. No expiry: the
  GM revokes.

## What a token reaches

`require_gm` reads `Authorization: Bearer` before the cookie. With a
bearer header, the matched route must be in
`auth::api_tokens::TOKEN_ROUTES`, checked before any database lookup;
anything else is 403 `TOKEN_NOT_ALLOWED`.

| Route | MCP tool |
| --- | --- |
| `GET /api/campaigns` | `list_campaigns` |
| `GET /api/campaigns/{id}` | `get_campaign`, `get_scene`, `get_readiness` (validator issues) |
| `GET /api/campaigns/{id}/readiness` | `get_readiness` (act gauges) |
| `POST /api/campaigns/{id}/story/edits` | `apply_story_edits` |
| `GET /api/campaigns/{id}/export` | `export_campaign_yaml` |
| `PUT /api/campaigns/{id}/import` | `import_campaign_yaml` |

Deliberately excluded: sign-in, accounts, GM invitations, the token
routes, creating / archiving / setting a campaign, `story/validate`
(declaring the campaign playable is the GM's decision; the validator's
report already comes with every read and edit), the table and players,
the live session and its socket, player and screen routes, and every
route that spends AI budget (generation, workshop, generated maps,
images, co-GM). `tests/gm_routes_test.rs` sends a live token to every
other GM route and expects 403; `tests/api_tokens_test.rs` covers the
in-scope routes, revocation and ownership (another GM's campaign is
404).

A token write is the GM's own write: same edit format, same lock, same
all-or-nothing, and players see it exactly as they would see the same
edit made on the review screen (only what the projection already
shows: title, universe, hook, revealed content).

## The MCP server (`mcp/`)

TypeScript on Bun with `@modelcontextprotocol/sdk`: the official SDK,
no build step (Bun runs the `.ts` entry directly), and the same runtime
as the rest of the repo's tooling. A Rust binary would have needed a
second, younger MCP crate and a compile on the Mac for no gain.

- Configuration: `PROMPTUS_URL` (the origin the app is opened at, no
  `/api`) and `PROMPTUS_TOKEN`. When either is missing the server still
  starts and every tool answers what to set.
- stdout is the protocol: logs go to stderr only.
- Errors come back as tool errors written in French for the GM:
  revoked token, out-of-scope route, unknown campaign, unreachable
  server, refused edit (with the server's `edit N: …` detail).
- Tests: `cd mcp && bun test` — a real MCP client over the SDK's
  in-memory transport, the API stubbed by a mocked `fetch`. CI runs them
  with `tsc --noEmit` (job `mcp`).

## Declaring it

Once per machine: `cd <checkout>/mcp && bun install`.

Claude Desktop — `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "promptus": {
      "command": "/Users/<you>/.bun/bin/bun",
      "args": ["run", "/path/to/DnD-promptus/mcp/src/index.ts"],
      "env": {
        "PROMPTUS_URL": "https://promptus.example",
        "PROMPTUS_TOKEN": "<token shown once>"
      }
    }
  }
}
```

Claude Desktop starts servers with a minimal `PATH`: give bun's full
path (`which bun`). Claude Code:

```bash
claude mcp add promptus -s user -e PROMPTUS_URL=https://promptus.example \
  -e PROMPTUS_TOKEN=<token> -- bun run /path/to/DnD-promptus/mcp/src/index.ts
```

The GM home shows both snippets with the app's origin, and with the
token filled in right after it is minted.
