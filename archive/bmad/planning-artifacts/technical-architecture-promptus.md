---
document: Technical Architecture
product: Promptus
status: Draft v1
date: 2026-04-28
author: Le Maitre
related: product-brief-promptus.md, brainstorming-session-2026-04-28-1123.md
---

# Promptus — Technical Architecture

## 0. Document Purpose

This document is the **technical blueprint** for Promptus' MVP. It translates the product vision (declarative narrative game engine) into concrete code organisation, data shapes, module boundaries, and engineering conventions. It is meant to be read once before coding begins, then consulted during implementation to keep all decisions consistent.

**Audience:** the implementer (Le Maitre, working with Claude Code).
**Scope:** Dictator Week prototype → MVP 3-5 months. Some V1 hooks are noted but not detailed.

---

## 1. High-Level Architecture

### 1.1 System diagram (logical)

```
┌─────────────────────────────────────────────────────────────────┐
│                       BROWSER (GM)                               │
│  Next.js Client Components (React + shadcn/ui)                  │
│  Zustand (UI/audio state) + TanStack Query (server data)        │
└────────────────┬────────────────────────────────────────────────┘
                 │ HTTPS (Caddy reverse proxy)
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    PROMPTUS APP (Home Server)                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Next.js Server (App Router + API Routes)               │   │
│  │  - Pages, RSC, Server Actions                           │   │
│  │  - REST API for entity CRUD, sessions, AI proxy         │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Game Engine (pure TS, framework-free)                  │   │
│  │  - Entity types, Effect resolver, Dice, Conditions      │   │
│  │  - World state mutator, Scene marker matcher            │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Drizzle ORM → Postgres 16 (Docker)                     │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Local /public/audio/   /public/generated-images/       │   │
│  └─────────────────────────────────────────────────────────┘   │
└────────────┬────────────────────────────────────────────────────┘
             │ HTTPS (outbound only)
             ▼
┌─────────────────────────────────────────────────────────────────┐
│  EXTERNAL AI                                                    │
│  HuggingFace Inference (Flux Schnell) — image generation        │
│  Later: fal.ai (Flux Pro) for production quality                │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  PLAYER PHONES (V1)                                             │
│  Read-only character sheet view (passive companion)             │
│  Phase 1: same Next.js app served as a "player view" route      │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Architectural style

- **Modular monolith** — single Next.js codebase, single deploy unit.
- **Engine isolated** — `lib/engine/*` is framework-free TypeScript, fully unit-testable, callable from API routes or UI.
- **Server-first data flow** — Postgres is the single source of truth; client cache via TanStack Query.
- **Event-traceable** — every engine resolution produces a `ResolutionRecord` for the timeline; no opaque side effects.

---

## 2. Code Organisation

```
promptus/
├── app/                          # Next.js App Router
│   ├── (marketing)/              # Landing, optional
│   ├── campaigns/
│   │   ├── [id]/
│   │   │   ├── page.tsx          # Campaign dashboard
│   │   │   ├── entities/
│   │   │   │   ├── page.tsx
│   │   │   │   └── [entityId]/edit/page.tsx
│   │   │   ├── audio/page.tsx
│   │   │   └── settings/page.tsx
│   │   └── new/page.tsx
│   ├── sessions/[id]/page.tsx    # The cockpit
│   └── api/
│       ├── campaigns/[id]/entities/route.ts
│       ├── entities/[id]/route.ts
│       ├── sessions/[id]/...
│       ├── ai/image/route.ts     # Image generation proxy
│       └── audio/route.ts
├── components/
│   ├── ui/                       # shadcn/ui primitives
│   ├── cockpit/
│   │   ├── PhaseSwitcher.tsx
│   │   ├── Hotbar.tsx
│   │   ├── InitiativeTracker.tsx
│   │   ├── ParticipantCard.tsx
│   │   ├── TimelineFeed.tsx
│   │   └── EntityPreview.tsx
│   ├── editor/
│   │   ├── EntityEditor.tsx
│   │   ├── EffectBuilder.tsx
│   │   └── YamlImportDialog.tsx
│   └── audio/
│       └── AudioPlayer.tsx
├── lib/
│   ├── engine/                   # ★ Pure game engine
│   │   ├── types.ts              # Entity, Effect, Condition, ResolutionRecord
│   │   ├── effects/
│   │   │   ├── damage.ts
│   │   │   ├── heal.ts
│   │   │   ├── apply-condition.ts
│   │   │   └── ... (one file per primitive)
│   │   ├── conditions/
│   │   │   ├── catalog.ts        # 14 standard conditions
│   │   │   └── apply-modifiers.ts
│   │   ├── dice.ts               # Notation parser + roller
│   │   ├── resolver.ts           # resolveEntity(), resolveEffect()
│   │   ├── markers.ts            # Scene marker condition matcher
│   │   └── world.ts              # World state mutator
│   ├── db/
│   │   ├── client.ts             # Drizzle client
│   │   ├── schema.ts             # All tables
│   │   └── migrations/
│   ├── ai/
│   │   ├── image-generator.ts    # Provider interface
│   │   ├── providers/
│   │   │   ├── huggingface.ts
│   │   │   └── fal.ts
│   │   └── prompt-template.ts    # Build prompts from style guide
│   ├── validation/
│   │   └── entity-schemas.ts     # Zod schemas mirror engine types
│   ├── stores/                   # Zustand
│   │   ├── audio-store.ts
│   │   └── session-store.ts
│   └── utils/
├── scripts/
│   ├── seed.ts                   # Demo data
│   ├── seed-srd.ts               # Future: import SRD 5.1
│   └── generate-srd-yaml.ts      # Future: pre-process SRD JSON
├── tests/
│   ├── unit/                     # Vitest, engine focus
│   └── e2e/                      # MCP Chrome / Playwright
├── public/
│   ├── audio/{ambiences,music,sounds}/
│   └── generated-images/
├── docker-compose.yml
├── Dockerfile
├── drizzle.config.ts
└── package.json
```

**Conventions:**
- Server-only modules: `import "server-only"` at top.
- Engine modules: zero imports from `next/*`, `react`, `db`. Pure TS. Mockable.
- Type imports: `import type { ... }` everywhere.
- File naming: `kebab-case.ts`. Components: `PascalCase.tsx`.

---

## 3. Data Model

### 3.1 Core tables (Postgres / Drizzle)

```typescript
// lib/db/schema.ts — sketch

export const campaigns = pgTable("campaigns", {
  id: text("id").primaryKey(),                    // nanoid
  name: text("name").notNull(),
  description: text("description"),
  styleGuide: jsonb("style_guide").$type<StyleGuide>().default({}),
  systemTemplate: text("system_template").default("dnd5e"),  // future-proof
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
});

export const entities = pgTable("entities", {
  id: text("id").primaryKey(),
  campaignId: text("campaign_id").references(() => campaigns.id, { onDelete: "cascade" }).notNull(),
  type: entityTypeEnum("type").notNull(),         // spell|item|npc|monster|character|location|event|condition|faction
  name: text("name").notNull(),
  description: text("description"),
  imageUrl: text("image_url"),
  tags: jsonb("tags").$type<string[]>().default([]),
  attributes: jsonb("attributes").$type<Record<string, unknown>>().default({}),
  effects: jsonb("effects").$type<Effect[]>().default([]),
  visibility: visibilityEnum("visibility").default("public"),
  version: integer("version").default(1),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
}, (t) => ({
  campaignTypeIdx: index("entities_campaign_type_idx").on(t.campaignId, t.type),
  nameIdx: index("entities_name_idx").on(t.name),
}));

export const sessions = pgTable("sessions", {
  id: text("id").primaryKey(),
  campaignId: text("campaign_id").references(() => campaigns.id, { onDelete: "cascade" }).notNull(),
  name: text("name").notNull(),
  currentPhase: phaseEnum("current_phase").default("exploration"),  // combat|dialogue|exploration
  initiativeOrder: jsonb("initiative_order").$type<InitiativeEntry[]>().default([]),
  combatRound: integer("combat_round").default(0),
  startedAt: timestamp("started_at").defaultNow().notNull(),
  endedAt: timestamp("ended_at"),
});

export const sessionState = pgTable("session_state", {
  id: text("id").primaryKey(),
  sessionId: text("session_id").references(() => sessions.id, { onDelete: "cascade" }).notNull(),
  entityId: text("entity_id").references(() => entities.id).notNull(),
  currentState: jsonb("current_state").$type<EntityState>().notNull(),  // hp, conditions, position, etc.
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
}, (t) => ({
  sessionEntityIdx: uniqueIndex("session_state_unique").on(t.sessionId, t.entityId),
}));

export const sessionTimeline = pgTable("session_timeline", {
  id: text("id").primaryKey(),
  sessionId: text("session_id").references(() => sessions.id, { onDelete: "cascade" }).notNull(),
  round: integer("round"),
  description: text("description").notNull(),       // "Bobby attacks Goblin: 18 vs AC 15 → 8 damage"
  resolutionRecord: jsonb("resolution_record").$type<ResolutionRecord>(),
  createdAt: timestamp("created_at").defaultNow().notNull(),
});

export const sceneMarkers = pgTable("scene_markers", {
  id: text("id").primaryKey(),
  campaignId: text("campaign_id").references(() => campaigns.id, { onDelete: "cascade" }).notNull(),
  name: text("name").notNull(),
  description: text("description"),
  conditions: jsonb("conditions").$type<MarkerCondition>().notNull(),
  effects: jsonb("effects").$type<Effect[]>().notNull(),
  status: markerStatusEnum("status").default("armed"),  // armed|triggered|disabled
  oneShot: boolean("one_shot").default(true),
  createdAt: timestamp("created_at").defaultNow().notNull(),
});

export const audioAssets = pgTable("audio_assets", {
  id: text("id").primaryKey(),
  name: text("name").notNull(),
  type: audioTypeEnum("type").notNull(),            // ambience|music|sound
  filePath: text("file_path").notNull(),
  durationSeconds: integer("duration_seconds"),
  tags: jsonb("tags").$type<string[]>().default([]),
  license: text("license"),
  attribution: text("attribution"),
});
```

### 3.2 Engine types (TypeScript)

```typescript
// lib/engine/types.ts — sketch

export type EntityType =
  | "spell" | "item" | "npc" | "monster"
  | "character" | "location" | "event" | "condition" | "faction";

export type Visibility = "public" | "mj_only" | "players_in_session" | "specific_users";

export interface BaseEntity {
  id: string;
  campaignId: string;
  type: EntityType;
  name: string;
  description?: string;
  imageUrl?: string;
  tags: string[];
  attributes: Record<string, unknown>;
  effects: Effect[];
  visibility: Visibility;
  version: number;
}

// Discriminated union for type-specific attributes
export interface SpellEntity extends BaseEntity {
  type: "spell";
  attributes: {
    level: number;
    school: string;
    castingTime: string;
    range: string;
    components: ("V" | "S" | "M")[];
    duration: string;
    save?: { stat: Stat; dcSource: string };
  };
}
// ... NpcEntity, MonsterEntity, ItemEntity, etc.

// 20 primitive effects (discriminated union)
export type Effect =
  | DamageEffect | HealEffect | ApplyConditionEffect | RemoveConditionEffect
  | ModifyStatEffect | RollCheckEffect | ConsumeResourceEffect | RestoreResourceEffect
  | SetStateEffect | MoveEntityEffect | RevealEntityEffect | SetRelationEffect
  | TriggerEventEffect | AddToInventoryEffect | RemoveFromInventoryEffect
  | PlayAmbienceEffect | PlayMusicEffect | PlaySoundEffect
  | DisplayImageEffect | DisplayTextEffect;

export interface DamageEffect {
  type: "damage";
  amount: string;                      // dice notation: "1d8+3", "8d6/2"
  damageType: DamageType;
  target: TargetSpec;
}

export interface ApplyConditionEffect {
  type: "apply_condition";
  conditionId: string;
  duration?: { rounds?: number; minutes?: number };
  target: TargetSpec;
  save?: { stat: Stat; dc: string | number };
}

// ... one interface per primitive

export type Stat = "STR" | "DEX" | "CON" | "INT" | "WIS" | "CHA";

export type TargetSpec =
  | { type: "self" }
  | { type: "single"; entityId: string }
  | { type: "all_in_area" }
  | { type: "caster" }
  | { type: "all_players" };

// What every effect resolution produces (for timeline)
export interface ResolutionRecord {
  effect: Effect;
  rolls: { notation: string; result: number; details?: string }[];
  outcome: "success" | "fail" | "partial" | "none";
  applied: { entityId: string; field: string; before: unknown; after: unknown }[];
  description: string;                 // Human-readable, for the timeline
  timestamp: string;
}
```

### 3.3 Marker conditions (structured)

```typescript
export type MarkerCondition =
  | { all: MarkerCondition[] }
  | { any: MarkerCondition[] }
  | { not: MarkerCondition }
  | { entityAttributeEquals: { entityId: string; attribute: string; value: unknown } }
  | { worldAttributeEquals: { attribute: string; value: unknown } }
  | { entityHasCondition: { entityId: string; conditionId: string } }
  | { questBranch: { questId: string; branch: string } };
```

A simple recursive matcher evaluates the tree against current world state on every state change. O(n) where n = condition tree size; trivially fast for hundreds of markers.

---

## 4. Engine Core — How Resolution Works

```typescript
// lib/engine/resolver.ts — sketch

export async function resolveEntity(
  entity: Entity,
  ctx: ResolutionContext,
): Promise<ResolutionRecord[]> {
  const records: ResolutionRecord[] = [];
  for (const effect of entity.effects) {
    const result = await resolveEffect(effect, ctx);
    records.push(result);
    // Some effects can trigger outcome branches
    if (result.outcome === "success" && effect.type === "roll_check") {
      for (const subEffect of effect.outcomeSuccess ?? []) {
        records.push(await resolveEffect(subEffect, ctx));
      }
    }
    if (result.outcome === "fail" && effect.type === "roll_check") {
      for (const subEffect of effect.outcomeFail ?? []) {
        records.push(await resolveEffect(subEffect, ctx));
      }
    }
  }
  return records;
}

export async function resolveEffect(
  effect: Effect,
  ctx: ResolutionContext,
): Promise<ResolutionRecord> {
  switch (effect.type) {
    case "damage":      return await effects.damage(effect, ctx);
    case "heal":        return await effects.heal(effect, ctx);
    case "apply_condition": return await effects.applyCondition(effect, ctx);
    // ... 20 cases
    default:            return notImplemented(effect);
  }
}
```

**`ResolutionContext`** carries everything the effect resolvers need:

```typescript
export interface ResolutionContext {
  sessionId: string;
  campaignId: string;
  caster?: Entity;
  targets: Entity[];
  worldState: WorldState;
  rng: () => number;                    // injectable for tests
  log: (event: string) => void;
  applyMutation: (m: Mutation) => Promise<void>;
}
```

**Mutations are explicit.** Every state change goes through `applyMutation`, which writes to `session_state` and emits to `session_timeline`. No effect mutates DB directly — separation between *deciding* and *applying* makes testing trivial.

---

## 5. Conditions Catalog

The 14 standard 5e conditions are **declarative**, not coded by case:

```typescript
// lib/engine/conditions/catalog.ts

export const CONDITIONS: Record<string, ConditionDefinition> = {
  paralyzed: {
    id: "paralyzed",
    name: "Paralyzed",
    description: "Incapable of action; auto-fails STR/DEX saves; attacks have advantage; melee hits are critical",
    modifiers: [
      { trigger: "incoming_attack", effect: "advantage" },
      { trigger: "outgoing_save", saveStat: ["STR", "DEX"], effect: "auto_fail" },
      { trigger: "incoming_attack_within_5ft", effect: "auto_critical" },
      { trigger: "incapacitated", effect: "true" },
    ],
  },
  poisoned: {
    id: "poisoned",
    name: "Poisoned",
    description: "Disadvantage on attacks and ability checks",
    modifiers: [
      { trigger: "outgoing_attack", effect: "disadvantage" },
      { trigger: "outgoing_check", effect: "disadvantage" },
    ],
  },
  // ... 12 more
};
```

The roller consults active conditions when computing a roll:

```typescript
function computeRollFlags(actor: Entity, target: Entity, kind: RollKind): { advantage: boolean; disadvantage: boolean } {
  let adv = false, dis = false;
  for (const cond of getActiveConditions(actor)) {
    const def = CONDITIONS[cond.id];
    for (const mod of def.modifiers) {
      if (matches(mod, kind, "outgoing")) {
        if (mod.effect === "advantage") adv = true;
        if (mod.effect === "disadvantage") dis = true;
      }
    }
  }
  for (const cond of getActiveConditions(target)) {
    // ... incoming triggers
  }
  return { advantage: adv && !dis, disadvantage: dis && !adv };
}
```

→ **Adding a new condition = adding entries to the catalog**, no code change to the engine.

---

## 6. AI Image Provider

```typescript
// lib/ai/image-generator.ts

export interface ImageGenerator {
  generate(prompt: string, opts: ImageOptions): Promise<ImageResult>;
}

export interface ImageOptions {
  width?: number;     // default 1024
  height?: number;    // default 1024
  guidanceScale?: number;
  seed?: number;
}

export interface ImageResult {
  url: string;        // local URL after download
  provider: string;
  promptUsed: string;
  latencyMs: number;
  costEstimate?: number;
}

// Provider selection via env
export function getImageGenerator(): ImageGenerator {
  switch (process.env.IMAGE_PROVIDER) {
    case "huggingface": return new HuggingFaceFluxSchnell();
    case "fal":         return new FalFluxPro();
    default:            return new HuggingFaceFluxSchnell();
  }
}
```

```typescript
// lib/ai/providers/huggingface.ts

export class HuggingFaceFluxSchnell implements ImageGenerator {
  private apiToken = process.env.HUGGINGFACE_TOKEN!;
  private endpoint = "https://api-inference.huggingface.co/models/black-forest-labs/FLUX.1-schnell";

  async generate(prompt: string, opts: ImageOptions): Promise<ImageResult> {
    const start = performance.now();
    const res = await fetch(this.endpoint, {
      method: "POST",
      headers: { Authorization: `Bearer ${this.apiToken}` },
      body: JSON.stringify({
        inputs: prompt,
        parameters: {
          width: opts.width ?? 1024,
          height: opts.height ?? 1024,
          guidance_scale: opts.guidanceScale ?? 3.5,
          num_inference_steps: 4,
        },
      }),
    });
    if (!res.ok) throw new ImageGenerationError(await res.text());
    const blob = await res.blob();
    const localUrl = await saveToPublic(blob);
    return {
      url: localUrl,
      provider: "huggingface/flux-schnell",
      promptUsed: prompt,
      latencyMs: performance.now() - start,
    };
  }
}
```

**Prompt building:**

```typescript
// lib/ai/prompt-template.ts

export function buildImagePrompt(entity: Entity, style: StyleGuide): string {
  const parts = [
    style.promptPrefix,                 // e.g. "dark fantasy, oil painting style"
    entity.description ?? entity.name,
    entity.tags.join(", "),
    style.promptSuffix,                 // e.g. "muted palette, dramatic lighting"
  ].filter(Boolean);
  return parts.join(", ");
}
```

---

## 7. Audio Architecture

```typescript
// lib/stores/audio-store.ts — Zustand

export interface AudioState {
  ambienceVolume: number;       // 0-1
  musicVolume: number;          // 0-1
  effectsVolume: number;
  currentAmbience?: AudioAsset;
  currentMusic?: AudioAsset;
  playAmbience: (a: AudioAsset) => void;
  stopAmbience: () => void;
  playMusic: (m: AudioAsset) => void;
  stopMusic: () => void;
  playSound: (s: AudioAsset) => void;
  stopAll: () => void;
}
```

The `<AudioPlayer>` component listens to the store and manages 3 HTML5 `Audio` elements (one per track type), with crossfade on switch.

---

## 8. API Routes (REST conventions)

```
GET    /api/campaigns                     List campaigns
POST   /api/campaigns                     Create campaign
GET    /api/campaigns/[id]                Campaign details
PATCH  /api/campaigns/[id]                Update campaign
DELETE /api/campaigns/[id]                Delete campaign

GET    /api/campaigns/[id]/entities       List entities (filter by type)
POST   /api/campaigns/[id]/entities       Create entity
POST   /api/campaigns/[id]/entities/import  Bulk YAML import
GET    /api/campaigns/[id]/entities/export  YAML export
GET    /api/entities/[id]                 Entity details
PATCH  /api/entities/[id]                 Update entity
DELETE /api/entities/[id]                 Delete entity

POST   /api/campaigns/[id]/sessions/start Start session
GET    /api/sessions/[id]                 Session state
PATCH  /api/sessions/[id]/phase           Change phase
POST   /api/sessions/[id]/initiative/roll Roll initiative for all
POST   /api/sessions/[id]/turn/next       Advance turn (decrement durations)
POST   /api/sessions/[id]/actions         Resolve action (attack/spell/effect)
GET    /api/sessions/[id]/timeline        Get timeline

POST   /api/ai/image                      Generate image (admin only)

GET    /api/audio/assets                  List audio library
POST   /api/audio/assets                  Upload custom audio (admin)
```

All endpoints validate via Zod, return typed JSON errors, log resolution records for audit.

---

## 9. State Management

| State kind | Tool | Why |
|---|---|---|
| Server data (entities, sessions) | TanStack Query | Cache, refetch, optimistic updates |
| Audio runtime (volumes, current) | Zustand | Local UI state, persists in localStorage |
| Form state (entity editor) | react-hook-form | Validation, dirty tracking |
| Cockpit ephemeral (selected entity, dialog open) | useState / useReducer | Component-local |

No Redux. No global store. Server data lives on the server; client caches what's needed.

---

## 10. Testing Strategy

### 10.1 Unit (Vitest)

- **Engine effects** — every primitive has tests for happy path, edge cases, target resolution. Mock `RNG`, mock `applyMutation`, assert on `ResolutionRecord`.
- **Dice notation parser** — `"1d20+5"`, `"2d6+3"`, `"8d6/2"`, advantage/disadvantage.
- **Conditions** — applying paralysed → next attack against target has advantage.
- **Marker matcher** — recursive condition trees evaluate correctly.

### 10.2 E2E (MCP Chrome / Playwright fallback)

Single full-session test (`tests/e2e/full-session.spec.ts`):
1. Seed → create campaign → import YAML
2. Generate one image
3. Start session → roll initiative
4. Attack → spell → condition
5. Advance turns → duration expires
6. Trigger ambience → assert audio playing
7. End session → assert timeline persisted

### 10.3 Manual smoke tests J7

Acceptance criteria document with screenshots for each foundation, executed before declaring Dictator Week done.

---

## 11. Deployment

### 11.1 Local dev

```bash
docker compose up -d                 # Postgres
pnpm install
pnpm db:migrate
pnpm seed:demo
pnpm dev                              # localhost:3000
```

### 11.2 Home server production

```bash
# On the server
git clone <repo>
cp .env.example .env.production       # fill HUGGINGFACE_TOKEN, DATABASE_URL
docker compose -f docker-compose.prod.yml up -d --build
```

### 11.3 docker-compose.prod.yml (sketch)

```yaml
services:
  db:
    image: postgres:16-alpine
    volumes: [pg_data:/var/lib/postgresql/data]
    environment:
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    restart: unless-stopped

  app:
    build: .
    depends_on: [db]
    environment:
      DATABASE_URL: postgresql://postgres:${DB_PASSWORD}@db:5432/promptus
      HUGGINGFACE_TOKEN: ${HUGGINGFACE_TOKEN}
      IMAGE_PROVIDER: huggingface
      NODE_ENV: production
    volumes:
      - ./public/audio:/app/public/audio
      - ./public/generated-images:/app/public/generated-images
    restart: unless-stopped

  caddy:
    image: caddy:2-alpine
    ports: ["80:80", "443:443"]
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile
      - caddy_data:/data
    restart: unless-stopped

volumes:
  pg_data:
  caddy_data:
```

Caddyfile:
```
promptus.your-domain.tld {
  reverse_proxy app:3000
}
```

---

## 12. Security Notes

- Single-user MVP, no auth. **Bind app port to `127.0.0.1`** behind Caddy; never expose Postgres port externally.
- API token for HuggingFace lives in `.env.production`, never committed.
- Image upload validated for MIME type and size (<10 MB).
- YAML import validated by Zod before any DB write — never `eval`, never executable content.
- Rate-limit `/api/ai/image` (30 req/hour) to avoid runaway costs.

---

## 13. Performance Considerations

- **Engine resolution** is in-process pure TS, p99 < 50 ms for typical effects.
- **Image generation** is the slow path: 5-10 s per image. UI shows explicit loading; client cancels via AbortController if user navigates away.
- **Postgres queries** all indexed; the only N+1 risk is loading session participants, mitigated with one JOIN-heavy query.
- **TanStack Query** dedupes refetches within 30 s; session timeline streams via polling every 3 s during active session (V1 will move to WebSocket).

---

## 14. Open Questions Deferred to V1

- WebSocket server for real-time push to player phones.
- Authentication (NextAuth.js with magic links) for multi-user.
- Voice command intake (Whisper API or Deepgram streaming).
- Dynamic relationship graph (N3) — schema sketch in section 3.1 already future-proof.
- Composition-style image generation (IP-Adapter / Flux Pro).
- Replay illustrated archive.
- SRD 5.1 bulk import script.

---

## 15. Architectural Decisions Recorded

| ADR | Decision | Rationale |
|---|---|---|
| ADR-001 | Modular monolith on Next.js | Solo dev, single deploy, simplest scaling for personal project |
| ADR-002 | Postgres direct from day 1 | Self-hosted, JSONB excellent for entities, future multi-user ready |
| ADR-003 | Engine in pure TS, framework-free | Testability, portability, separation of concerns |
| ADR-004 | Declarative effect catalog over coded rules | Scales linearly with content, AI can generate, GM can extend |
| ADR-005 | No runtime AI text in MVP | GM uses Claude Code subscription out-of-band; cost = 0 |
| ADR-006 | External image API only | No GPU on home server; abstraction allows swap without code change |
| ADR-007 | Curated audio catalog, no AI generation | Cost control, deterministic UX, license clarity |
| ADR-008 | Drizzle ORM | Type safety, migrations, SQLite + Postgres compatible (future flexibility) |
| ADR-009 | shadcn/ui over component lib | Code in repo, fully customizable, no version drift |
| ADR-010 | Zustand + TanStack Query split | Right tool for each kind of state; no Redux |

---

## 16. Reading Order for the Implementer

1. This document (architecture, ~30 min)
2. `product-brief-promptus.md` (vision, ~10 min)
3. `dictator-week-plan.md` (execution, ~15 min)
4. Start J1 with the schema in section 3.1 in front of you
