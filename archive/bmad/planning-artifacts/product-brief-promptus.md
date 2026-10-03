---
document: Product Brief
product: Promptus
status: Draft v1
date: 2026-04-28
author: Le Maitre
---

# Product Brief: Promptus

> *« The stage is set. Let the session begin. »*

## Executive Summary

**Promptus** is a tabletop RPG platform built on a declarative game engine that treats stories, rules, and characters as structured, playable objects. It is positioned not as a virtual tabletop — there are already several of those — but as a **narrative stage director** for game masters who care more about immersion and pacing than mechanical simulation.

The product is built for first-time and intermediate GMs running D&D-style sessions with friends. It eliminates the cognitive overhead of running a session by automating the parts a GM should not have to think about (combat resolution, world state, asset retrieval) while staying invisible during the parts that matter (roleplay, improvisation, dramatic moments).

Three things make Promptus distinct: a **declarative engine** that handles both rules (Pokémon-style L3 combat) and narration (world state, scene markers, dynamic relationships); a **Streamdeck-style cockpit** that adapts to the current phase of play; and a **content philosophy** where the GM keeps absolute control while the platform supplies the props (images, ambient sound, NPC sheets) on cue.

## The Problem

Running a D&D session is exhausting work, especially for newcomer GMs. Marc, our archetypal user, is 32, has 4-5 hours per week to prepare, has never run a campaign before, and is the only one in his friend group willing to be GM. He needs to track HP, conditions, spell slots, NPC dispositions, scene transitions, and ambience — while also voicing characters, improvising, managing pacing, and making sure his friends are having fun.

Existing tools force a choice between two extremes:

- **D&D Beyond** is excellent for character sheets but does not run sessions. Marc still needs another tool for atmosphere, encounter management, and persistence.
- **Foundry VTT** simulates the table beautifully but assumes a power-user who enjoys configuring modules for hours. The learning curve is too steep for Marc's first session next Sunday.
- **Roll20** sits in between with a tired UX, manual everything, and no generative content.
- **Alchemy RPG** focuses on narrative play but has no generative AI, no co-creation system, and a small content catalog.

The result: Marc abandons digital tools, prints PDFs, and relies on memory. His friends notice the seams. Sessions feel improvised in the wrong way. The platform that could have made him a great GM never gets built.

## The Solution

Promptus is built around a single architectural decision: **the declarative game engine is the platform, not a feature of it.** Every entity in the world — spells, items, NPCs, locations, conditions, scene events — is a structured declaration the engine can resolve, mutate, and track.

This unlocks four things at once:

1. **Combat resolves automatically** at L3 depth (jets, damage, conditions applied mechanically) without coding hundreds of special cases. The engine reads each spell or attack as a composition of primitive effects (`damage`, `apply_condition`, `roll_check`, etc.).

2. **Story state stays consistent.** Killed NPCs are marked dead. Visited locations are remembered. Faction dispositions update when players act. Marc cannot accidentally resurrect the merchant he killed three sessions ago.

3. **Scene markers give the GM a safety net.** Pre-prepared events ("when players enter the crypt and the moon is red, the Revenant appears") wait quietly until conditions are met. Promptus alerts the GM, the GM decides whether to trigger.

4. **AI generation becomes structurally meaningful.** When the GM uses Claude Code (their personal subscription, outside the runtime app) to generate an NPC, the output is a structured YAML the engine can immediately use — not loose prose to copy-paste.

The runtime experience is a **Streamdeck-style cockpit** that morphs by phase — Combat, Dialogue, Exploration — exposing only the actions relevant to the current moment. Voice triggers handle atmospheric effects (ambience switches, image reveals at dramatic peaks) but never the system itself: keyboard fallback is permanent.

## What Makes This Different

Promptus does not compete with Foundry on simulation depth or with D&D Beyond on character sheet sophistication. It occupies an empty slot in the market: **the first narrative game engine for tabletop RPGs**.

- **Declarative narration** (world state, scene markers, dynamic relationships) is a category no existing tool offers natively.
- **Asymmetric architecture** — GM-app is critical and resilient, player-app is passive and optional — reflects how real friend groups actually play, including those with one tech-averse member.
- **Voice-first only for theatrical effects**, never for system commands, sidesteps the failure mode that has buried voice control in every other VTT attempt.
- **Content seeded from SRD 5.1** plus AI-generated supplements (NPCs, locations) gives day-one playability without the legal risk of redistributing Wizards material.
- **Self-hostable** on a home server with minimal external dependencies (image generation API only). No vendor lock-in, no monthly subscription required for personal use.

The unfair advantage is **clarity of identity**: every product decision passes a single test — *does this help the director run a better show?* If the answer is no, the feature does not ship.

## Who This Serves

**Primary user — Marc, the novice GM (the north star).**
Mid-30s, has watched Critical Role, has been drafted as GM by friends, has zero patience for tools that require a weekend tutorial. He wants to feel competent in front of his friends. He wants the prep to be enjoyable rather than draining. He wants the session to flow even when he forgets a rule or hesitates on a description.

**Secondary user — Léa, the immersion-first player.**
Loves theater of mind, dislikes screens taking over the table, wants the technology to disappear during play. Promptus respects her by making the player app optional, passive, and never intrusive (no narrative vibrations, no notification spam).

**Tertiary user — Tom, the optimizer.**
Knows the rules cold, values mechanical depth. Promptus serves him with on-demand stat consultation, declarative spell mechanics, and modular advanced rules he can opt into without forcing the rest of the table to deal with them.

**The platform builder — Le Maitre (you).**
Personal project first, with a path to broader use if it earns one. The brief reflects this: ambitious in vision, pragmatic in execution, no commercial pressure that would distort design.

## Success Criteria

For a personal project, traditional product metrics are noise. The honest signals are:

1. The engine runs a 3-hour session without crashing. *(Foundation reliability.)*
2. The first session with Promptus requires less than 30 minutes of GM setup. *(Onboarding.)*
3. At least one friend says "let's do this again" after the first session. *(Desirability.)*
4. The friend group asks for Promptus sessions instead of avoiding them. *(Voluntary adoption.)*
5. GM prep time drops below 1 hour per session thanks to the platform. *(Prep ROI.)*
6. At least 3 complete sessions are played without abandonment. *(Endurance.)*
7. The builder still enjoys working on Promptus after 3 months. *(Sustainability.)*

## Scope

### In for the prototype week ("Dictator Week")

The prototype is ambitious by design — every one of the four foundations is shipped properly rather than half-built:

- **Preparation:** Visual entity editor (NPC, location, item, spell, monster) backed by the declarative schema. YAML import/export for entities generated outside the app via Claude Code.
- **Session runtime:** Streamdeck-style cockpit with three phase modes (Combat / Dialogue / Exploration). L3 combat resolution (initiative tracker, dice automation, condition badges with mechanical effects, HP/resource tracking).
- **Image:** Generative image creation tied to entities (NPCs, monsters, locations) via HuggingFace Inference + Flux Schnell. Style guide per campaign.
- **Sound:** Curated ambient audio library (locations, combat music, mood beds) from royalty-free sources; manual triggering from the cockpit.

### In for the broader MVP (3-5 months)

- Narrative engine N1+N2 (world state tracking, scene markers/triggers).
- Single starter scenario: *"The Goblin Dungeon"*, 3-hour one-shot, level 1.
- Content seed: SRD 5.1 import (~250 spells, monsters, items), 14 conditions, 8-10 location ambiences, 5-8 functional music tracks, 10-15 NPC and location templates.
- Player-side passive companion view (read-only character sheet, ambient audio reception).

### Out of MVP scope

- Voice control. *(V1 — Whisper or Deepgram, GM-only mic.)*
- Composition-style live image generation at climaxes. *(V1 — Flux Pro + IP-Adapter.)*
- Player-driven lore co-creation. *(V1 — whitelist permissions, MJ validation.)*
- Dynamic relationship graph (N3). *(V1.)*
- Replay archive with illustrated timeline. *(V1.)*
- Multi-table support for a single GM. *(V2.)*
- Synthetic NPC voices, AI co-GM, async narrative play. *(Permanently excluded — anti-features.)*

## Technical Approach

- **Stack:** Next.js 15 (App Router) + TypeScript strict, Tailwind + shadcn/ui, Zustand + TanStack Query, Drizzle ORM on Postgres, Vitest + MCP Chrome for tests.
- **Engine:** Declarative entities (YAML/JSON) resolved by a primitive-effect catalog (~20 primitives covering combat, narration, sensory output).
- **AI text:** None at runtime. The GM uses their own Claude Code subscription out-of-band to generate entities, then imports YAML into Promptus.
- **AI image:** External API (HuggingFace + Flux Schnell for MVP, fal.ai + Flux Pro for production). Provider abstracted behind an interface.
- **Audio:** Curated royalty-free catalog stored locally on the home server.
- **Hosting:** Home server (own hardware). Postgres + Next.js app run side-by-side. Image generation always offloaded to external API (no local GPU).
- **Cost ceiling:** ~10 €/month in production (image API + domain).

## Vision

In two to three years, Promptus is the answer to *"how do I run a great session for my friends without becoming a project manager?"* It supports system-agnostic content, hosts a community-curated catalog of scenarios, allows GMs to share campaigns without exposing their secrets, and quietly runs in the background of friend group rituals across thousands of homes.

The category — *narrative game engine for tabletop RPGs* — is not a fight Foundry will pivot into and not a niche Alchemy will reach. The path is clear: ship the prototype this week, validate with the founder's own friend group within a month, expand the SRD content base over the next quarter, and let the rest emerge from real usage.

The platform stays true to its founding constraints: the GM has absolute control, the AI helps in prep but never takes the stage, the screens serve the story rather than steal it. If those discipline lines hold, Promptus becomes the tool first-time GMs reach for instinctively — not because they were sold on it, but because it makes them feel like the storyteller they hoped to be.

---

## Appendix A — Anti-features (permanent)

The following are explicitly out of Promptus, today and forever:

1. Synthetic NPC voices (the GM voices their own characters).
2. AI live narrative suggestions (no co-GM that "directs the scene").
3. Asynchronous narrative play (D&D is a synchronous social game).
4. Foundry-level rule simulation (L5 — multiclass interaction, full spell catalog auto, lair actions parallel resolution).
5. Between-session world simulation (NPCs do not act on their own while the table sleeps).
6. Auto-cliffhangers and "smart" return of forgotten NPCs (the GM directs the story).
7. Forced player journals.
8. Paper sheet OCR (digital sheets only).

## Appendix B — Six design rules

Every product decision passes through these:

1. **The GM has absolute control.** The machine proposes, never decides.
2. **AI lives in prep, not in live narration.**
3. **The platform fades behind roleplay.** If a feature distracts from friend conversation, it is downsized.
4. **Marc the novice is the north star.** Every feature is judged against his first session.
5. **Screens are tools, not the game.**
6. **Narrative identity outranks system identity.** The engine stays invisible.
