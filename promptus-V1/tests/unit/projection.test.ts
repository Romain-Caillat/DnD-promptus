import { describe, it, expect } from "vitest";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { normalizeWorld } from "@/lib/engine/world";
import { projectPlayerView, type ProjectionInput } from "@/lib/play/projection";
import { buildDemoStory } from "../../scripts/demo-story";

const story = buildDemoStory((n) => `e:${n}`);
const ent = (name: string, type: string, visibility = "mj_only", attributes: Record<string, unknown> = {}) => ({
  id: `e:${name}`, name, type: type as never, description: `${name} desc`, imageUrl: null, visibility, attributes,
});
const entities = [
  ent("Bobby", "character", "public", { abilityScores: { STR: 16 }, speed: 30 }),
  ent("Léa", "character", "public"),
  ent("Squelette", "monster"),
  ent("Zombie", "monster"),
  ent("Revenant Mort-Roi", "monster"),
  ent("Sœur Mira", "npc"),
];

function input(over: Partial<ProjectionInput> = {}): ProjectionInput {
  return {
    session: { id: "s", name: "Partie", currentPhase: "exploration", combatRound: 0, activeTurnIndex: 0, initiativeOrder: [] },
    campaignName: "Démo",
    story,
    world: { ...normalizeWorld(null), currentSceneId: "sc_crypte" },
    ruleset: DND5E_RULESET,
    entities,
    states: [
      { entityId: "e:Bobby", currentState: { hp: 9, hpMax: 12, ac: 16, conditions: [{ conditionId: "poisoned", remainingRounds: 2 }] } },
      { entityId: "e:Léa", currentState: { hp: 7, hpMax: 7, conditions: [] } },
      { entityId: "e:Squelette", currentState: { hp: 13, hpMax: 13, conditions: [] } },
    ],
    player: { id: "p1", name: "Romain", characterEntityId: "e:Bobby" },
    takenBy: { "e:Bobby": "Romain" },
    timeline: [],
    requests: [],
    ...over,
  };
}

describe("projectPlayerView — confidentialité", () => {
  it("ne divulgue ni résumé, ni notes MJ, ni secrets, ni bible", () => {
    const v = projectPlayerView(input());
    const json = JSON.stringify(v);
    const scene = story.scenes.find((s) => s.id === "sc_crypte")!;
    expect(v.scene).toEqual({ title: scene.title, readAloud: scene.readAloud, phase: "combat" });
    expect(json).not.toContain(scene.summary);
    for (const secret of story.bible.secrets) expect(json).not.toContain(secret);
    expect(json).not.toContain("cl_");
  });

  it("ne montre que les cases et pions révélés", () => {
    const hidden = projectPlayerView(input());
    expect(hidden.map?.id).toBe("map_crypte");
    expect(hidden.map?.cells).toEqual([]);
    expect(hidden.map?.tokens).toEqual([]);

    const world = {
      ...normalizeWorld(null),
      currentSceneId: "sc_crypte",
      mapState: { map_crypte: { revealed: ["4,5", "5,12", "0,0"] } },
    };
    const v = projectPlayerView(input({ world }));
    expect(v.map?.cells.map((c) => `${c.x},${c.y}`).sort()).toEqual(["0,0", "5,12"]); // mur + escalier, rien d’autre
    expect(v.map?.tokens.map((t) => t.label)).toEqual(["Adversaire"]); // squelette en (4,5), nom caché
  });

  it("dévoile le nom d’un adversaire une fois révélé", () => {
    const world = {
      ...normalizeWorld(null),
      currentSceneId: "sc_crypte",
      revealedEntityIds: ["e:Squelette"],
      mapState: { map_crypte: { revealed: ["4,5"] } },
    };
    expect(projectPlayerView(input({ world })).map?.tokens[0].label).toBe("Squelette");
  });
});

describe("projectPlayerView — fiche, tour et actions", () => {
  it("donne la fiche du personnage avec états et caractéristiques du ruleset", () => {
    const v = projectPlayerView(input());
    expect(v.character?.hp).toBe(9);
    expect(v.character?.conditions).toEqual([{ id: "poisoned", label: "Empoisonné", remainingRounds: 2 }]);
    expect(v.character?.abilities.find((a) => a.id === "STR")).toMatchObject({ abbr: "FOR", score: 16, modifier: 3 });
    expect(v.party.map((p) => p.name)).toEqual(["Bobby", "Léa"]);
  });

  it("propose les actions de la phase en cours", () => {
    const v = projectPlayerView(input({ session: { ...input().session, currentPhase: "dialogue" } }));
    expect(v.actions.map((a) => a.label)).toContain("Persuader");
    expect(v.actions.map((a) => a.label)).not.toContain("Attaquer");
  });

  it("limite le déplacement au tour du joueur en combat", () => {
    const world = {
      ...normalizeWorld(null),
      currentSceneId: "sc_crypte",
      mapState: { map_crypte: { revealed: ["5,12"], tokens: [{ entityId: "e:Bobby", x: 5, y: 12 }] } },
    };
    const order = [
      { entityId: "e:Squelette", initiative: 15, isPlayer: false },
      { entityId: "e:Bobby", initiative: 10, isPlayer: true },
    ];
    const notMine = projectPlayerView(input({ world, session: { ...input().session, currentPhase: "combat", combatRound: 1, activeTurnIndex: 0, initiativeOrder: order } }));
    expect(notMine.isMyTurn).toBe(false);
    expect(notMine.movement).toMatchObject({ allowed: false, reason: "Ce n’est pas votre tour", budgetCells: 6 });
    expect(notMine.initiative.map((i) => i.name)).toEqual(["Adversaire", "Bobby"]);

    const mine = projectPlayerView(input({ world, session: { ...input().session, currentPhase: "combat", combatRound: 1, activeTurnIndex: 1, initiativeOrder: order } }));
    expect(mine.isMyTurn).toBe(true);
    expect(mine.movement?.allowed).toBe(true);
  });
});
