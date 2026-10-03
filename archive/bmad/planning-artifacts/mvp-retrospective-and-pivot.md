---
document: MVP Retrospective + V1 Pivot Notes
product: Promptus
date: 2026-04-28
author: Le Maitre
status: To revisit before resuming work
related:
  - product-brief-promptus.md
  - technical-architecture-promptus.md
  - ../brainstorming/brainstorming-session-2026-04-28-1123.md
---

# Promptus — MVP Retrospective and V1 Pivot Notes

## TL;DR

The Dictator Week MVP shipped in a single afternoon (J1 → J7 condensed in
~3-4 hours of Claude Code time). Code is committed in `promptus/` —
working app at <http://localhost:3001> with a 48-entity demo campaign,
combat L3 resolver, AI image hooks, audio engine and YAML import/export.

**However — the MVP is not yet a usable product for a real session.** It
covers the mechanical layer (combat, tracking, atmosphere) but does not
support the 80% of a tabletop session that is narration, NPC roleplay,
exploration descriptions, scene transitions and continuity between
sessions. The cockpit helps the GM resolve dice; it does not help the GM
tell a story.

The decision: **stop adding features on top of the current MVP, take a
fresh angle, and re-plan with gameplay and narration as the primary
design driver.**

## What we built

Implementation summary (commits in `/Users/iliad/02_perso/D&D/promptus/`):

| Day | Commit  | Scope |
|-----|---------|-------|
| J1  | 767aaf2 | Next.js 16 + Postgres + Drizzle + entity schema + CRUD + seed |
| J2  | 5da75c6 | Entity editor + 20-effect builder + YAML import/export |
| J3  | 6b1e00e | Streamdeck cockpit + initiative + participant cards + hotbar |
| J4  | 000e03c | Combat L3 resolver + dice + 14 conditions + attack/cast dialogs + timeline |
| J5  | befb4f1 | HuggingFace Flux Schnell + style guide + cockpit reveal |
| J6  | eb78ef1 | Pixabay audio catalog + player + cockpit controls + engine wiring |
| J7  | dd35f68 | Demo seed (48 entities) + Docker prod + Caddy + README |

42 unit tests passing. Build successful on Next.js 16 standalone output.

## What is missing for a real session — narration gap

The MVP supports the GM **as a referee**, not **as a narrator**. Concrete
gaps surfaced during the J7 review:

1. **No scene/scenario authoring.** There is no place to write "what is
   supposed to happen in this scene" — only loose entities. The GM must
   still hold the entire scenario in their head or in a separate document.

2. **No live note-taking surface.** During play, things change (Bobby
   accepted the quest, Léa lied to the mayor, the party promised the
   blacksmith). Today the cockpit has no place to capture those.

3. **NPC dialogue is unsupported.** When the GM enters dialogue phase,
   nothing in the UI helps them voice the NPC: no quick access to
   motivation, secret, voice note, or recently-said-things. The
   participant cards are tuned for combat (HP/AC), not roleplay.

4. **Exploration is unsupported.** When the party enters a new location,
   the GM cannot quickly read the description aloud. The location image
   reveals work, but the text is buried.

5. **No quest/objective tracking.** Even a simple "what is the party
   trying to do right now" reminder is missing.

6. **No session-to-session continuity surface.** Reopening a session
   gives no recap; the GM is dropped into a state with no narrative
   bookmark.

7. **Inventory exists in the schema but does not surface in cockpit.**
   Items can be added to a character via raw_effects but there's no
   convenient view-and-equip flow.

8. **Dialogue and exploration hotbar actions are placeholders.** Combat
   has working buttons (attack, cast, roll initiative); the other
   phases toast "not yet wired".

These gaps mean the MVP can simulate combat well, but the surrounding
80% of a session — exploration, NPC interaction, narrative beats — is
left entirely to the GM's notebook.

## What we learned about velocity

The single most important meta-insight from this afternoon:

**Claude Code with a clean schema and disciplined tasks shipped in
~3-4 hours what we estimated at 56-70h.** The multiplier on cadence is
roughly ×15-20 in this regime.

This changes the calculus for V1. The original V1 estimate of 6-9 months
realistically maps to **6-12 weeks of solo dev**, possibly less if the
narration-first scope replaces some of the originally-listed features.

The bottleneck is no longer engineering speed. The bottleneck is
**knowing what to build**.

## Critical reminder — narration is part of the engine, not a layer above

A foundational design decision from the brainstorm was that **narration
lives inside the declarative game engine**, not as a separate UI layer
on top. Specifically (Vision-Cores #92-#97):

- **N1 — World state tracking**: every entity has mutable state attributes
  (alive/dead, location, disposition, status). The engine reads and
  writes these.
- **N2 — Scene markers / triggers**: pre-declared events with conditions
  (`if X then suggest Y`). The engine watches conditions and notifies
  the GM when triggers are ready to fire.
- **N3 — Dynamic relations**: NPCs/factions have disposition graphs
  that the engine mutates when actions resolve.
- N4-N5-N6 (rigid quests, emergent simulation, live AI co-GM) are
  permanently excluded.

**What the MVP already has, that supports this:**

- ✅ `scene_markers` table exists in the DB schema (`src/lib/db/schema.ts`)
  with `conditions`, `effects`, `status: armed|triggered|disabled`,
  `oneShot`. Not used yet — no UI surfaces it, no engine code matches
  conditions.
- ✅ The 20 primitive effects already include 5 narrative ones:
  `set_state`, `move_entity`, `reveal_entity`, `set_relation`,
  `trigger_event`. Resolver code paths for them exist.
- ✅ Entity attributes carry narrative fields (`status`, `motivation`,
  `secret`, `currentLocation`, `faction` on NPCs) used in the seed.
- ✅ Session state tracks `currentState` per entity — already a small
  world-state.

**What the MVP is missing on the narrative-engine side:**

- ❌ No condition matcher for scene markers — the engine doesn't watch
  them, the GM never sees a "marker armed" notification.
- ❌ No surface to author scene markers in the UI (the schema accepts
  them but no editor / form / picker).
- ❌ No relations/disposition data structure between entities (N3).
  The `set_relation` effect resolves but there is no place to read or
  visualize the resulting graph.
- ❌ No automatic state propagation. Killing an NPC marks them dead in
  session_state, but the entity's own attribute is not updated, and no
  scene marker depending on "NPC X is dead" would notice.
- ❌ Scenes themselves don't exist as a separate concept yet — only
  as scene markers. We probably want a **Scene** entity (read-aloud
  text + present NPCs + linked location + active markers) as a
  first-class engine entity.

So the next iteration should not say "add narration on top of the
engine". It should say **"finish wiring the narrative half of the
engine that we already designed but didn't ship"**, plus surface it in
the cockpit with the editing and consultation UIs the GM needs.

The data model question (point 3 below) becomes narrower: probably one
new table `scenes`, plus extending NPCs with an `inScene` reference,
plus a relations table (or jsonb graph on each entity). Notes can be a
small `session_notes` table — they don't need to be engine entities.

## What we want to revisit before resuming

When we come back to this project, **start by re-asking, not by coding**.
Specifically:

### 1. Re-anchor the product on narration, not on combat

The original brainstorm correctly identified narration as the
differentiator (Promptus is "régisseur narratif augmenté", not
"simulateur de table"). The MVP execution drifted toward combat
because the engine + resolver + dice are the most concrete things to
build first. The next iteration should resist that gravity.

A useful re-framing question: **"What does the GM see when nobody is
attacking anybody?"** Today the answer is: not much. The next pass
should make that the primary screen, with combat as an interruption
mode.

### 2. Run a play-style brainstorm rather than a feature brainstorm

The phase 1-4 brainstorm we did was excellent for vision but stopped at
features. What we missed: **a session walkthrough**. Pick a real D&D
session you'd run with your friends and trace each minute:

- 0:00–0:05 — players arrive, GM sets the scene, what does the screen
  show?
- 0:05–0:20 — exploration, descriptions, atmosphere, what does the GM
  consult?
- 0:20–0:40 — first NPC dialogue, how does the GM keep the NPC
  consistent?
- 0:40–1:00 — combat, current MVP handles this
- 1:00–1:30 — return to roleplay, decisions, branching
- 1:30 — session ends, what does the GM save for next time?

Every minute that has a question mark over "what does the cockpit do
here" is a missing feature. That list will be more honest than any
abstract feature list.

### 3. Extend the engine's narrative half — don't bolt it on

The engine already has the narrative primitives (`set_state`,
`move_entity`, `reveal_entity`, `set_relation`, `trigger_event`) and
the `scene_markers` table. What's missing is more declarative content
on top of those primitives, plus a few new entity kinds that the
engine treats as first-class:

- **Scene** — a narrative unit declared once, instantiated in a
  session. Holds: read-aloud text, present NPC ids, current location
  id, active scene markers, transitions to next scenes. The engine
  knows how to "enter" a scene (sets state, plays ambience, reveals
  NPCs) and "exit" a scene.
- **Quest / objective** — small, declarative state (active, complete,
  failed) that scene markers can read and write. Probably a new entity
  type rather than a separate table.
- **Note** (free-form, GM-only) — *not* an engine concept. Just a
  small `session_notes` table. The engine doesn't reason about notes.
- **Recap** — derived from `session_timeline` + `session_state`
  diffs at session end. Not a primitive; a query plus a Claude prompt.
- **Relations / disposition** — needs a real data structure
  (a relations table or a jsonb graph on faction entities). The
  `set_relation` effect needs a place to write to.

So the schema work is small in scope: 2 new tables (`scenes`,
`session_notes`), 1 new entity type (`quest`), one extension to NPC
attributes (`inScene` reference), and a relations storage choice.

The harder work is in the **scene authoring and condition-matching
loop**: building an editor for scene markers, wiring a condition
matcher into the action resolver so that mutations on entity state
surface "marker armed" suggestions to the GM in the cockpit.

### 4. Reconsider the cockpit layout

The Streamdeck-style cockpit is right for combat. It is wrong for
narration. A narrative cockpit probably wants:

- a large, central, calm panel for the **current scene's read-aloud
  text and image**
- an editable note-taking strip permanently visible
- the present NPCs as quickly-accessible cards with voice/motivation
- combat + initiative as a **transient overlay** that opens when
  combat starts and closes when it ends

The current architecture (phase modes + hotbar) can absorb this, but
the visual hierarchy needs to flip: narration first, mechanics on
demand.

### 5. Reconsider what "useful in 1 session" means

The MVP success criterion was "engine runs a 3h session without
crashing". That criterion was met but it set the bar too low. A better
criterion: **"the GM finished the session feeling that Promptus made
their job easier than running it from a notebook would have."**

If we don't have confidence the answer is yes, we don't have a useful
product yet.

## What to keep from the current MVP

When we re-plan, **do not throw away** what already works:

- ✅ The entity schema and 20-effect resolver are solid foundations.
  They do their job; they just need narrative concepts layered on top.
- ✅ The Drizzle + Postgres setup, the Zod validation everywhere, the
  pure-TS engine isolation — all good architectural choices to keep.
- ✅ The audio catalog and image generation hooks work and are reusable
  as-is.
- ✅ The combat L3 resolver with dice, conditions, advantage/disadvantage
  is genuinely useful for the moments combat happens; we don't need to
  rewrite it.
- ✅ The Streamdeck pattern is good for a *combat* phase; it just
  shouldn't be the dominant chrome.

Treat the next session as **"add a narrative layer on top of a working
mechanical engine"**, not as "redo from scratch".

## Concrete next-session plan (when we resume)

When we come back to this:

1. **Hold a play-walkthrough session** (you and me, ~1h) before any
   coding, mapping a real session minute-by-minute against the cockpit.
2. **Identify 5-8 narrative primitives** the cockpit must expose
   (scene, note, npc-quick-access, location-read-aloud, quest, recap…).
3. **Re-derive the data model additions** (likely: `scenes`, `notes`,
   `quests` tables, plus extended NPC fields).
4. **Design the new cockpit layout** as a wireframe before any
   component change. Goal: narration-first.
5. **Implement in one or two focused sprints** using the same Claude
   Code velocity we proved today.

## Files to revisit when we return

- `/Users/iliad/02_perso/D&D/promptus/` — the working MVP
- `_bmad-output/brainstorming/brainstorming-session-2026-04-28-1123.md`
  — the full vision (97 fragments, 6 pillars, 5 killer features)
- `_bmad-output/planning-artifacts/product-brief-promptus.md`
  — the executive brief
- `_bmad-output/planning-artifacts/technical-architecture-promptus.md`
  — the engine-level design
- This document — the gap analysis and re-planning anchor

## The one-line reminder

> **The MVP proves the engine works. The next iteration must prove that
> the GM's job got easier — and that is a narration problem first, a
> mechanics problem second. Narration is part of the same engine, not
> a UI veneer; the next sprint finishes the narrative half of the
> engine that's already partially wired.**
