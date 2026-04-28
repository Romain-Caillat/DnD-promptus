# Promptus — Claude Code prompt template

Use this prompt with [Claude Code](https://claude.ai/code) (or claude.ai)
to generate entities that import cleanly into Promptus via the YAML
importer (`/campaigns/[id]/entities` → "Import YAML").

---

## Prompt

> You are populating the world of a Promptus campaign — a tabletop RPG
> platform whose core abstraction is a declarative game engine. Your output
> must be a single multi-document YAML stream of entities, separated by
> `---`. Each entity must conform to the schema below and use only the 20
> primitive effects listed.
>
> **Entity schema** (per document):
> ```yaml
> type: spell | item | npc | monster | character | location | event | condition | faction
> name: string
> description: string  # narrative paragraph; players see this when visibility allows
> tags: [array of short strings, lower-case]
> attributes:
>   # type-specific fields below
> effects: [array of primitive effects]
> visibility: public | mj_only | players_in_session | specific_users
> ```
>
> **Type-specific attribute hints:**
> - `spell` → `level: int`, `school: string`, `castingTime: string`,
>   `range: string`, `components: [V, S, M]`, `duration: string`
> - `monster` / `character` → `hp: int`, `hpMax: int`, `ac: int`,
>   `speed: int`, `challengeRating: number`, `size: string`,
>   `abilityScores: { STR, DEX, CON, INT, WIS, CHA }`,
>   `initiativeBonus: int`
> - `npc` → `hp`, `hpMax`, `ac`, `faction: string`,
>   `currentLocation: string`, `status: alive|wounded|dead|missing`,
>   `motivation: string`, `secret: string` (mj_only),
>   `voiceActorRecommended: string`
> - `item` → `rarity: common|uncommon|rare|very_rare|legendary`,
>   `weight: number`, `value: number`, `attunement: bool`
> - `location` → `parentLocation: string`,
>   `defaultAmbienceId: string` (matches an audio_assets id or tag)
>
> **The 20 primitive effects** (each one is an object with `type` plus its
> arguments):
>
> 1. `damage` — `{ amount: dice, damageType, target }`
> 2. `heal` — `{ amount: dice, target }`
> 3. `apply_condition` — `{ conditionId, duration: { rounds | minutes }, target, save? }`
> 4. `remove_condition` — `{ conditionId, target }`
> 5. `modify_stat` — `{ stat, modifier: int, duration?, target }`
> 6. `roll_check` — `{ stat: STR|DEX|CON|INT|WIS|CHA, dc: int, target,
>     outcomeSuccess: [Effect[]], outcomeFail: [Effect[]] }`
> 7. `consume_resource` — `{ resource: spell_slot|ki|rage|..., amount: int, level?: int, target }`
> 8. `restore_resource` — same shape as consume_resource
> 9. `set_state` — `{ entityId, attribute: string, value }`
> 10. `move_entity` — `{ entityId, toLocationId }`
> 11. `reveal_entity` — `{ entityId, toUsers: all_players|mj_only|[ids] }`
> 12. `set_relation` — `{ fromId, toId, disposition?: string, delta: int }`
> 13. `trigger_event` — `{ eventId }`
> 14. `add_to_inventory` — `{ itemId, target, quantity?: int }`
> 15. `remove_from_inventory` — same shape
> 16. `play_ambience` — `{ ambienceId }`
> 17. `play_music` — `{ musicId }`
> 18. `play_sound` — `{ soundId }`
> 19. `display_image` — `{ imageId? } | { prompt? }`
> 20. `display_text` — `{ text }`
>
> **Targeting** (used inside any effect's `target` field):
> ```yaml
> target:
>   type: self | caster | single | multiple | all_in_area | all_players | all_enemies
>   # When type=single
>   entityId: ent_xxx     # leave empty if to be picked at runtime
>   # When type=multiple
>   entityIds: [ent_a, ent_b]
> ```
>
> **Dice notation**: `1d20+5`, `2d6+3`, `8d6/2` (the `/2` halves the total —
> use it for "half on save" cascades), `1d10-2`, plain `5` for a flat
> number.
>
> **Standard 5e conditions** you may apply: blinded, charmed, deafened,
> frightened, grappled, incapacitated, invisible, paralyzed, petrified,
> poisoned, prone, restrained, stunned, unconscious.
>
> **Style requirements:**
> - Use lower-case `damageType` (`fire`, `slashing`, `piercing`, `acid`,
>   `cold`, `lightning`, `thunder`, `poison`, `psychic`, `necrotic`,
>   `radiant`, `force`, `bludgeoning`).
> - For spells with a save, prefer the `roll_check` cascade pattern with
>   `outcomeFail` (full effect) and `outcomeSuccess` (mitigated effect).
> - Attribution: tag every NPC/monster you create with the campaign or
>   region they belong to.
>
> Now generate the entities I describe below. Output **ONLY** the YAML — no
> introductory text, no closing remarks, no markdown fences.
>
> ## My request:
>
> {{describe what you want — e.g. "5 cultists for an undead-themed dungeon
> tier 1, plus the cultist boss as an NPC, plus 3 ambient location
> descriptions"}}

---

## Tips

- Run the prompt once for the bulk you need, paste the output into the YAML
  importer dialog, and Promptus will validate every entity before inserting.
- Invalid entities are rejected with a precise error message — fix and
  re-import only the broken docs.
- For complex scenarios, iterate: ask for monsters first, then for NPCs
  that reference those monsters by id, then for locations.
- Always review generated stats for balance — Claude is good but not
  infallible. Compare with the SRD for tier-1 references.
