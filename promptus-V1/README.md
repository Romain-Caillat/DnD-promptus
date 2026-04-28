# Promptus

> *The stage is set. Let the session begin.*

A narrative game engine for tabletop RPGs — a stage director for game masters
who care about immersion and pacing more than mechanical simulation.

Promptus is built around a single architectural decision: **the declarative
game engine is the platform, not a feature of it.** Every entity in the world
— spells, items, NPCs, locations, conditions, scene events — is a structured
declaration the engine can resolve, mutate, and track.

## What it does today

- **Streamdeck-style cockpit** with phase modes (Combat / Dialogue /
  Exploration / Travel / Rest).
- **L3 combat resolver** — initiative, attack rolls vs AC, advantage and
  disadvantage from active conditions, critical hits including the
  paralyzed-target-within-5ft rule, doubled damage dice on crit, demi-damage
  via `8d6/2` notation.
- **20 primitive effects** that compose into spells, items and scene events:
  damage, heal, apply/remove condition, modify stat, roll check (with
  cascading outcome branches), consume/restore resource, set state, move
  entity, reveal entity, set relation, trigger event, inventory, ambience,
  music, sound, image, text.
- **14 standard 5e conditions** declared as data, automatically applied
  when computing roll flags.
- **Generative images** via HuggingFace + Flux Schnell, with a per-campaign
  visual style guide (art style, mood, palette, prompt prefix/suffix).
- **Royalty-free audio catalog** (8 ambiences, 5 music tracks, 4 SFX) from
  Pixabay, with automatic dispatching of `play_ambience` / `play_music` /
  `play_sound` effects emitted by the resolver.
- **YAML import/export** for entities — bulk import, multi-doc support, full
  schema validation.
- **Self-hostable** on a home server. No vendor lock-in, no monthly
  subscription required.

## Quickstart — local development

```bash
git clone <your-fork>
cd promptus
pnpm install
docker compose up -d           # Postgres on :5433
cp .env.example .env.local
pnpm db:migrate                # apply schema
pnpm seed:all                  # audio catalog + demo campaign
pnpm dev                        # http://localhost:3000
```

Open `/campaigns` to see "The Goblin Dungeon — Demo" with 4 PCs, 15 spells,
10 monsters, 10 items, 5 locations, 4 NPCs.

### Optional: AI image generation

Promptus uses HuggingFace's free Flux Schnell endpoint for image generation.
Without a token it remains fully usable — only the "Generate" button on the
entity editor is unavailable.

1. Create a free account at <https://huggingface.co>.
2. Generate a "Read" token at <https://huggingface.co/settings/tokens>.
3. Accept the model terms at
   <https://huggingface.co/black-forest-labs/FLUX.1-schnell>.
4. Add the token to `.env.local`:
   ```
   HUGGINGFACE_TOKEN=hf_xxxxxxxxxxxx
   ```
5. Restart the dev server.

## Deploying to your home server

Promptus ships a multi-stage Dockerfile and a production compose file with
Postgres + Caddy reverse proxy + automatic HTTPS.

```bash
ssh you@home-server
git clone <your-fork>
cd promptus
cp .env.production.example .env.production
$EDITOR .env.production            # set DB_PASSWORD; HUGGINGFACE_TOKEN if needed
$EDITOR Caddyfile                  # set your domain
docker compose -f docker-compose.prod.yml --env-file .env.production up -d
```

Inside the running app container, apply migrations the first time:

```bash
docker exec -it promptus-app sh -c "node node_modules/.bin/drizzle-kit migrate"
```

Caddy will obtain a Let's Encrypt certificate as soon as DNS resolves.

## Generating entities with Claude Code

Promptus' MVP intentionally has **no runtime AI for text generation** — you
use your existing Claude Code (or Claude.ai) subscription to author entities
out-of-band, then drop the YAML into Promptus' importer.

A copy-pasteable prompt template lives in [`docs/claude-code-prompt.md`](docs/claude-code-prompt.md).
The short version:

> Generate a YAML document conforming to the Promptus entity schema. The
> schema and effect catalog are documented in the file. The output must be a
> single multi-document YAML stream separated by `---`. Validate that every
> effect's `type` is one of the 20 primitives.

## Architecture

```
┌──────────────────────── Browser (GM) ────────────────────────┐
│ Next.js client + Tailwind + shadcn/ui                        │
│ Zustand (UI state), TanStack Query (server state)            │
└─────────────────┬────────────────────────────────────────────┘
                  │
┌─────────────────▼─── Promptus app (home server) ─────────────┐
│ Next.js 16 (App Router) + standalone build                   │
│ Pure-TS engine in /lib/engine                                │
│ Drizzle ORM → Postgres 16                                    │
│ Audio catalog: /public/audio (or external CDN)               │
└─────────────────┬────────────────────────────────────────────┘
                  │ HTTPS (outbound only)
                  ▼
                  HuggingFace Inference (Flux Schnell, optional)
```

### Stack

- **Next.js 16** + React 19 + TypeScript (strict)
- **Tailwind 4** + shadcn/ui (Radix preset Nova)
- **Drizzle ORM** + Postgres 16
- **Zod** for input validation everywhere
- **Vitest** for unit tests, MCP Chrome for E2E
- **Pure-TS engine** isolated in `lib/engine/` with no framework imports

## Roadmap

### Today (MVP — shipped)
- Game engine with 20 primitive effects
- L3 combat resolution with 14 conditions
- Streamdeck cockpit with 5 phases
- AI image generation (Flux Schnell)
- Audio catalog with 17 royalty-free assets
- YAML import/export

### V1 (next 6 months)
- Voice control for atmospheric effects (Whisper or Deepgram)
- Composition-style live image generation at climax moments (Flux Pro +
  IP-Adapter or ControlNet)
- Player-driven lore co-creation with whitelist permissions
- Dynamic relationship graph (N3 — "Liens du destin")
- Session replay with illustrated timeline
- SRD 5.1 bulk import for full content seed

### V2+ (post product/market fit)
- Multi-table support for one GM
- Multi-GM collaborative campaigns
- Marketplace of community scenarios
- Modular advanced rules (opportunity attacks, cover, elevation)

### Permanent anti-features
- Synthetic NPC voices (the GM voices their own characters)
- AI live narrative suggestions (no co-GM that "directs the scene")
- Asynchronous narrative play
- Foundry-level rule simulation

## Six design rules

Every product decision passes through these:

1. **The GM has absolute control.** The machine proposes, never decides.
2. **AI lives in prep, not in live narration.**
3. **The platform fades behind roleplay.** If a feature distracts from
   friend conversation, it is downsized.
4. **Marc the novice is the north star.** Every feature is judged against
   his first session.
5. **Screens are tools, not the game.**
6. **Narrative identity outranks system identity.** The engine stays
   invisible.

## License

Personal project. Audio assets ship under the Pixabay Content License
(commercial use OK, no attribution required but appreciated). Promptus' own
code is your own — treat this fork as a starting point, not a product.

## Acknowledgments

- Pixabay artists for the seeded audio.
- Black Forest Labs for releasing Flux Schnell.
- Wizards of the Coast SRD 5.1 (CC-BY-4.0) for the rules grammar this engine
  is shaped around (without redistributing protected content).
