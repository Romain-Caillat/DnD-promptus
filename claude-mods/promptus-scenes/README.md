# promptus-scenes

A Claude Code mod (`platform/preview-campaign-in-claude`): a pane beside
the conversation that shows a campaign's scenes, act by act, with their
summary and their assets, and what is left to produce before the
evening. Read-only: it never writes a campaign.

## What it shows

- Each scene on one line: map, music chosen / planned, opponents,
  missing fields; `nouvelle` / `modifiée` when it changed during the
  session.
- The selected scene: summary, location, mood, hook, exits, each asset
  (map found in `content/maps/`, YouTube tracks chosen or still to
  search, illustration and NPC portraits described), the scene fields
  the readiness gauge expects, and « inventé — à valider ».
- « À produire avant la soirée »: the gaps of the act or campaign.

It reads the campaign files (`content/campaigns/*/*.yaml`, and
`content/fixtures/*.yaml` as examples), not the campaigns stored in
Promptus. It refreshes when Claude edits a YAML under `content/`, and
every few seconds when the file's modification time changes.

## Load it

Needs `ruby` on the machine (its standard YAML library parses the
campaign). For one session:

```bash
claude --plugin-dir ~/02_perso/DnD-promptus/claude-mods/promptus-scenes
```

For every session, including the desktop app, add the folder to
`CLAUDE_CODE_PLUGIN_DIRS` in the `env` block of `~/.claude/settings.json`.

Then `/promptus` opens the pane; `/promptus corsaires` picks a campaign;
`/promptus <folder>` points it at a Promptus checkout when the session
runs elsewhere (default: the session's folder, then
`~/02_perso/DnD-promptus`).

Function hooks are an early-access Claude Code API: built and validated
on Claude Code 2.1.286 (`claude plugin validate .`).

## Layout

- `hooks/register.tsx` — the pane, the `/promptus` command, the refresh.
- `hooks/scan-rb.ts` — the Ruby reader that turns a campaign into the
  summary the pane draws.
- `types/index.d.ts` — the pane's state contract.
