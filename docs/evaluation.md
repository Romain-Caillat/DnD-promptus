# Evaluating the AI on the real games

`ai/evaluate-on-real-campaigns`: a prompt or a model that changes must
not quietly break what worked. `bun run eval` replays a fixed set of
cases to the configured model (`.env`: `OPENROUTER_API_KEY`,
`OPENROUTER_MODEL`), scores each answer on written criteria, saves the
run in `evals/runs/<date>.json` (and the report as `.md`), and prints
what improved or regressed since the previous run, case by case.

```bash
bun run eval            # the configured model; costs real money
bun run eval -- --fake  # the deterministic provider: tries the bench, saves nothing
```

Run it after every change of prompt template (`back/prompts/`) or of
model, and commit the runs worth keeping: the next run compares itself
with the newest one in `evals/runs/`.

## Cases

One YAML file per case in `content/evals/`, drawn from the two games
(`docs/lecons-des-parties.md` §4):

| Case | What it replays |
|------|-----------------|
| `refuser-offre-vaubernier` | the table refuses Vaubernier's offer; he answers in character and keeps the escort secret |
| `incident-jacquot` | Jacquot spills his mug on the Bretteur; the consequence points to the hull clue |
| `combat-du-quai` | three of Gueule-Rouge's sailors are down; the scene's morale rule plays out |
| `generer-un-acte-corsaire` | a campaign generated from a pitch, held to the scene standard and the validator |

A case names a campaign (relative to `content/`) and either a
`copilot` situation — the scene, the scenes entered, the clues found,
the names known, the journal, and the GM's request — or a `generation`
pitch. Then its `criteria`, each with an `id`, a French `why` (what
the report prints) and a `check`:

| Check | Passes when |
|-------|-------------|
| `speaks: <npc id>` | that NPC has a line |
| `never: [words]` | none of the words reaches the players (narration, NPC lines) |
| `mentions: [words]` | one of the words is anywhere in the answer |
| `suggests: { type: reveal_clue, clue: <id> }` | a suggestion carries that action |
| `no_invented_ids` | nothing the model named had to be dropped |
| `no_dice` | no dice formula reaches the players |
| `french` | what reaches the players reads as French |
| `no_errors` | the generated campaign has no validator error |
| `three_clue_rule` | no three-clue alert in the generated campaign |
| `scene_standard` | every generated scene has the scene standard's fields |

Words are matched case-insensitively, as substrings.

## How a run calls the model

Straight through the provider, with the same prompts as the server
(`copilot::messages`, `prep::generation::generate`): no database, no
campaign budget, no ledger row. The report gives what the run cost.
A call that fails (unreachable, out of format) fails every criterion
of its case, with the reason.

## Comparing

A case is *en progrès* when more of its criteria pass, *en recul* when
fewer do — or as many but not the same ones — and each criterion that
moved is marked *(corrigé)* or *(cassé)*.
