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
non-blocking check (`engine/lint-rule-system`).

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
  - { id, name, description, tier?, abilities, armor_class, hit_points, actions, tactics, note }
```

A class must give a score for every ability. An adversary's armour class
and hit points are stated, not derived. Action ids are unique across the
whole system (they key cooldowns on a sheet).

## Not yet in the format

Vehicles (`engine/support-vehicle-combat`) and death saves played out
(`engine/save-against-death`). Ship combat's action economy is already
expressible as a turn context.
