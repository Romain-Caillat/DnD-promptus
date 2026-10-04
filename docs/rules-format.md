# Rule system format

A rule system is one YAML file, `content/rules/<id>/v<version>.yaml`. It
is the only copy of a campaign's rules: the engine (`shared/src/rules/`),
the player's rules page, the action cards and the co-GM prompt all read
it. Changing a rule means editing this file and saving it as a new
version; a campaign points at one `(id, version)`.

Loading (`RuleSystem::from_yaml`) refuses what the engine cannot run —
an unknown field (typos included), a reference to an ability, condition,
action kind, difficulty, situation, item or tier that does not exist, a
formula that does not evaluate, levels out of order. Each error has a
stable `code`, a `path` (`classes[bretteur].actions[estocade].tags[2]`,
or a line and column for a syntax error) and an English `detail`; the
UI translates the code. Whether the rules are *good* is a separate,
non-blocking check (`rules::lint`, see *Lint* below).

Ids are `snake_case` (abilities are written as on the sheet: `FOR`,
`DEX`…). Game text — names, descriptions, notes — is French.

The two witness systems, `corsaires/v1.yaml` and `brasier/v1.yaml`, are
faithful transcriptions of Romain's games, defects included; read them
as worked examples. Comments marked `INTERPRETATION` are readings the
model forced where the source was ambiguous.

## Top level

| Key | What it is |
| --- | --- |
| `id`, `version`, `name`, `description` | Identity; `version` starts at 1 |
| `sources` | Documents the version was transcribed from |
| `abilities` | `{ id, name, description }`, at least one |
| `modifier` | Formula from `score` to the modifier |
| `stats.armor_class`, `stats.hit_points` | `{ name, abbr, formula }` over abilities and `level` |
| `check` | `{ dice: 1d20, advantage: bool }` — one die; `advantage: false` forbids any advantage |
| `difficulties` | Named thresholds `{ id, name, value }` |
| `outcomes` | The four bands (below) |
| `group_check` | Optional: `{ succeeds_when: at_least_half \| majority \| all \| any }` |
| `attack` | `{ ability: first_primary \| best_primary, precision: added_to_attack_roll \| not_applied }` |
| `initiative` | `{ dice, bonus: formula, ties: party_first \| reroll }` |
| `action_kinds` | What a turn spends: `{ id, name, cost }` |
| `turn_contexts` | Action economy per setting (below) |
| `cooldowns.meaning` | `skip_next_turns` or `turn_of_use_counts` (below) |
| `durations.application_turn_counts` | Whether the turn a condition lands in counts (below) |
| `progression` | `{ upgrade_every_xp, upgrade_points, levels: [{ level, xp }] }` |
| `zero_hp` | What 0 hit points does (below) |
| `creation` | `{ abilities: from_class, free_action_slots }` |
| `movement` | Per map scale (`world`, `place`, `encounter`): `cells_per_move` (`null` when unstated) and optional `grid: { diagonal: chebyshev\|alternate, difficult_factor, climb_cost, max_step, swim_factor }` — the `maps::MovementRules` the grid reads |
| `combat` | Optional: fighting on a grid — move and flight actions, cover, long range, zone size (below) |
| `situations` | Circumstances the GM declares (`furtif`, `en_hauteur`) |
| `resources` | `{ id, name, abbr, start }` (gold…) |
| `conditions`, `classes`, `items`, `adversary_tiers`, `adversaries` | Below |
| `peoples` | Playable peoples (empty in both drafts) |

## Formulas

Integers, `+ - * /`, parentheses, `mod(X)`, `score(X)`, `min(a, b)`,
`max(a, b)`, and the variables `score` (modifier formula only) and
`level`. **`/` rounds down** (towards minus infinity): with
`(score - 10) / 2`, a 9 gives −1.

## Rolls and outcome bands

A roll is the check die plus modifiers against a target (a difficulty,
an armour class, or the other side's total in a contest). The band:

1. a natural face listed in `critical_failure.natural` → critical failure;
2. a natural face listed in `critical_success.natural` → critical success;
3. otherwise total ≥ target → success, else failure; no target → no band.

Each band has a `name`, a `description` and `grants: { xp }`.
`critical_success.damage_multiplier` multiplies an attack's damage.
Every roll returns its breakdown: faces, kept face, each modifier and its
source, total, target, band.

## Turns, cooldowns, durations

A `turn_context` gives `actions_per_turn` and `limits` such as
`{ kind: attaque, max_per_turn: 1 }`, plus `references` to documents it
sends the reader to. A scene runs in one context.

`cooldown: N` on an action (0 = none):

- `skip_next_turns` — unusable for the rest of the turn and the user's
  next N turns (cooldown 1 = every other turn);
- `turn_of_use_counts` — the turn of use is one of the N (cooldown 1 =
  not twice in a turn).

Condition durations count the **bearer's** turns and tick at their end.
With `application_turn_counts: false`, a condition put on someone during
their own turn (a self buff) does not lose a turn there.

## Zero hit points

```yaml
zero_hp: { rule: knocked_out, condition: inconscient, out_after_turns: 3, out_condition: hors_combat }
# or
zero_hp: { rule: death_saves, condition: mourant, difficulty: 10, successes: 3, failures: 3 }
```

`knocked_out`: a heal lifts `condition`; after `out_after_turns` of the
bearer's turns without one, `out_condition` replaces it.
`death_saves` is modelled, played by `engine/save-against-death`.

## Fighting on a grid

The combat layer (`shared/src/combat/`) reads the map for range, sight,
cover, areas and paths. What the rules say about it:

```yaml
combat:
  move_kind: deplacement            # action kind one move spends; absent = one free move per turn
  flee: { kind: fuite, ability: DEX } # absent = fleeing spends every action left, no roll
  cover: { half: -2, three_quarters: -5 }  # default; added to attack rolls
  long_range: { modifier: -2 }      # default; or `disadvantage` (needs `check.advantage`)
  zone_radius: 1                    # default; cells around a zone's centre
```

- **Initiative** is `initiative.dice` + `initiative.bonus` for everyone,
  highest first, the order kept for the whole fight. Ties: `party_first`
  puts the party first (then the order combatants were placed in);
  `reroll` rerolls the tied ones among themselves.
- **A move** spends one `move_kind` action and covers `cells_per_move`
  of the map's scale, scaled by `movement_percent` conditions, along a
  path checked step by step (`maps::check_path`: walls, doors, enemies,
  corners, climbing, water). When `cells_per_move` is `null` (both
  witness worlds), the engine uses **6 cells** — 9 m on an encounter map,
  the D&D 5e walk, the same source as the grid's movement defaults.
- **Range** is in cells, with the system's diagonal rule. An action's
  `range` (absent = 1, adjacent only) is how far its target, a zone's
  centre or a line may reach; up to its optional `long_range` it still
  goes, with the `long_range` penalty.
- **Sight**: touching cells always see each other; farther, the line
  must be clear (`maps::line_of_sight`) and within the map's
  `sight_limit` (fog). The target's cover adds `cover.half` or
  `cover.three_quarters` to the attack roll (shown in the breakdown as a
  `cover` modifier); total cover means no line of sight.
- **Areas** are resolved from the grid, enemies only: `melee_burst`
  catches every enemy adjacent to the actor; `zone` every enemy within
  `zone_radius` of a visible centre in range; `line` every enemy within
  `range` on the line towards the aimed cell, walls stopping it.
- **Leaving the fight**: an adversary at 0 HP is defeated at once (the
  zero-HP rule is read for player characters); a character the rule
  puts out of the scene leaves; a knocked-out one stays on the ground
  (crossable, not a cell to stop on) until healed or out; a flight
  succeeds unless the GM asks for the `flee.ability` roll and it fails.
- **The end**: when a side has nobody in the fight above 0 HP, or when
  the GM stops it. The summary lists the defeated, the fled, those out
  of the scene and the XP each character gained (from their rolls'
  bands; no separate victory XP exists in the format yet).

## Conditions

```yaml
- id: empoisonne
  name: Empoisonné
  kind: bane            # or boon — the badge colour
  description: "-1 à tous ses jets."
  effects: [{ roll_modifier: { rolls: all, value: -1 } }]
```

Effects: `skips_turn`, `cannot_move`, `{ movement_percent: 50 }`,
`{ roll_modifier: { rolls: all|checks|attacks|saves, value } }`,
`{ advantage: { rolls } }`, `{ disadvantage: { rolls } }`,
`advantage_against`, `disadvantage_against`, `{ precision: n }`,
`{ precision_against: n }`, `{ damage_dealt: n }`, `{ damage_taken: n }`,
`ignore_damage`, `{ ability_bonus: { ability?: X, value } }` (all
abilities when absent), `{ damage_per_turn: dice }`,
`{ on_hit: <apply> }`, `ends_when_attacked`, `{ narrative: "…" }` — the
last is shown to the GM and never computed.

## Actions

Class actions, adversary attacks and item uses share one shape:

```yaml
- id: bombe_a_meche
  name: Bombe à mèche
  description: Lance un explosif artisanal. Touche une zone.
  kind: attaque           # an action_kinds id: what the turn spends
  level: 3                # class actions only; absent = always available
  target: enemies         # self | ally | ally_or_self | enemy | enemies | all_allies
  area: zone              # melee_burst | zone | line — required for `enemies`
  roll: attack            # none | attack | auto_hit | auto_critical | { contest: { actor: CHA, target: SAG } }
  ability: DEX            # optional; else attack.ability over the class's primaries
  tags:
    - { damage: { amount: 3 } }        # a number or dice: 3, 1d6+2
    - { control: { condition: etourdi, turns: 1, to: targets, save: { ability: SAG } } }
    - { cooldown: 3 }
```

Tags — they are the card's chips, and the engine's data:

- `damage: { amount, note? }`, `heal: { amount, note? }` — on each
  target the action lands on; a critical multiplies damage.
- `buff` / `control`: an apply — `{ condition: id }` or inline
  `{ effects: [...], kind: boon|bane }`, plus `turns`, `to: targets|self`,
  optional `save: { ability, difficulty? }` (no difficulty = the GM sets
  it when it happens) and `note`. `to: self` applies once, whatever the
  rolls.
- `precision: n`, `cooldown: n` — at most one each.
- `situational: { situation, precision?, damage? }` — only when the GM
  declares that situation.
- `choice: [tag, tag]` — the player picks one option.
- `note: "…"` — what the engine does not compute; shown to the GM.

Optional on any action: `range: <cells>` and `long_range: <cells>`
(see *Fighting on a grid*).

An action is refused, with nothing changed, when it is not the actor's
turn, a condition keeps them from acting, their level is too low, it is
on cooldown, the turn budget or a kind limit is spent, an item is
missing, or the targets do not fit.

## Classes, items, adversaries

```yaml
classes:
  - { id, name, description, primary_abilities: [FOR, DEX], secondary_abilities: [],
      abilities: { FOR: 13, … }, items: [{ item, qty }], actions: [...], notes: [] }
items:
  - { id, name, description, price?, consumable?, action?, note? }
adversary_tiers:
  - { id: soldat_entraine, name: Soldat entraîné, armor_class: 12 }
adversaries:
  - { id, name, description, tier?, abilities, armor_class, hit_points, actions, tactics, note, exception? }
```

A class must give a score for every ability. An adversary's armour class
and hit points are stated, not derived. The lint compares a tiered
adversary's armour class with its tier, and an untiered one's with the
`stats.armor_class` formula; `exception: "<raison>"` (non-empty) says
the difference is intended and silences both for that stat block. Action ids are unique across the
whole system (they key cooldowns on a sheet).

## Lint

`rules::lint(&system)` (or `lint_with` for a given campaign) checks
whether the rules are *good*. It never blocks: a system with issues
loads and plays. Each issue has a `severity` (`error`, `warning`,
`info`), a stable `code`, a `path` in the style above, an English
`detail` and a French `message` for the GM — the same shape as the
campaign validator's issues (`promptus_shared::issue::Issue`).

| Code | Severity | What it flags |
| --- | --- | --- |
| `DAMAGE_MODEL_MIXED` | warning | players, NPCs, items or conditions do not share one damage model (fixed vs dice); the players' model is the reference |
| `PRECISION_NOT_APPLIED` | warning | precision values on cards or conditions while `attack.precision` is `not_applied` |
| `PRIMARY_ABILITY_AMBIGUOUS` | warning | `attack.ability: first_primary` with classes that attack and have several primaries |
| `ADVERSARY_AC_OFF_TIER` | warning | a tiered adversary's AC is not its tier's (unless `exception`) |
| `ADVERSARY_AC_FORMULA` | warning | an untiered adversary's AC is not the AC formula's (unless `exception`) |
| `REFERENCE_MISSING` | error | a `references` entry, or a file name cited in free text, is not among the `sources` |
| `UNDEFINED_TERM` | warning | free text compares with something undefined (« par rapport à une épée standard ») or names a roll that does not exist (« jets de soin ») |
| `NAME_DUPLICATE` | warning | two items, or two different class actions, share a name |
| `COOLDOWN_UNEXPLAINED` | info | cooldowns are used and `cooldowns.note` does not say what they mean |
| `TURN_CONTEXTS_DIFFER` | info | a turn context's budget or limits differ from the first context's — the GM confirms |
| `ABILITY_TOTAL_OUTLIER` | warning | a class's ability total differs from the median class's |
| `DAMAGE_PER_TURN_LOW` | info | a damage-dealing class deals under half the median at level 1 |
| `PROGRESSION_MAX_EARLY` | warning | a class reaches the top level before the campaign's last session |

The balance numbers behind the last three come from
`rules::balance_report(&system, &BalanceParams)`: per class, the ability
total, the expected damage per turn at each level against each
adversary tier (or the median class AC when there is no tier), the XP
expected per session and the level at the end of the campaign.
`BalanceParams` sets the campaign (default: 4 sessions, 2 fights of 4
rounds and 6 checks per player each session, checks against the median
difficulty, the first turn context). The numbers are expectations, not
a simulation: one target, base scores, no situational bonus or buff,
each class playing its best sustainable mix of unlocked actions under
the turn budget, kind limits and cooldowns; a contest counts as a check
against 11. `Display` prints it as a plain table.

## Not yet in the format

Vehicles (`engine/support-vehicle-combat`) and death saves played out
(`engine/save-against-death`). Ship combat's action economy is already
expressible as a turn context.
