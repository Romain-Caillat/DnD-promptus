import { describe, it, expect } from "vitest";
import { newContext, resolveEffects } from "@/lib/engine/resolver";
import { EMPTY_WORLD, evaluateCondition, sceneTriggers, normalizeWorld } from "@/lib/engine/world";
import { buildDemoStory } from "../../scripts/demo-story";

const story = buildDemoStory((name) => `ent:${name}`);

function ctxWith(world = normalizeWorld(EMPTY_WORLD), catalog = new Map()) {
  return newContext({ entities: [], initiativeOrder: [], story, world, catalog });
}

describe("evaluateCondition", () => {
  it("une révélation est connue dès qu’un de ses indices est trouvé", () => {
    const w = { ...normalizeWorld(null), foundClueIds: ["cl_greta"] };
    expect(evaluateCondition({ revelationKnown: "rv_aldo" }, w, story)).toBe(true);
    expect(evaluateCondition({ revelationKnown: "rv_crypte" }, w, story)).toBe(false);
    expect(evaluateCondition({ not: { revelationKnown: "rv_crypte" } }, w, story)).toBe(true);
  });
  it("une scène résolue compte comme visitée", () => {
    const w = { ...normalizeWorld(null), sceneStatus: { sc_bois: "resolved" as const } };
    expect(evaluateCondition({ sceneStatus: { sceneId: "sc_bois", status: "visited" } }, w, story)).toBe(true);
  });
});

describe("effets narratifs", () => {
  it("entrer dans une scène la rend courante et visitée", async () => {
    const ctx = ctxWith();
    const { records } = await resolveEffects([{ type: "enter_scene", sceneId: "sc_village" }], ctx);
    expect(ctx.world.currentSceneId).toBe("sc_village");
    expect(ctx.world.sceneStatus.sc_village).toBe("visited");
    expect(records[0].description).toContain("Creux-d’Étain sous la peur");
  });

  it("faire avancer une menace applique les effets de l’étape atteinte, une seule fois", async () => {
    const ctx = ctxWith();
    const first = await resolveEffects([{ type: "advance_front", frontId: "fr_gobelins", steps: 2 }], ctx);
    expect(ctx.world.frontProgress.fr_gobelins).toBe(1);
    // étape 0 sans effet, étape 1 avec un display_text
    expect(first.records.map((r) => r.effect.type)).toEqual(["advance_front", "display_text"]);
    const again = await resolveEffects([{ type: "advance_front", frontId: "fr_gobelins", steps: -1 }], ctx);
    expect(ctx.world.frontProgress.fr_gobelins).toBe(0);
    expect(again.records).toHaveLength(1);
  });

  it("l’horloge est bornée à la dernière étape", async () => {
    const ctx = ctxWith();
    await resolveEffects([{ type: "advance_front", frontId: "fr_mortroi", steps: 99 }], ctx);
    expect(ctx.world.frontProgress.fr_mortroi).toBe(3);
  });

  it("révéler un indice annonce la révélation la première fois seulement", async () => {
    const ctx = ctxWith();
    const a = await resolveEffects([{ type: "reveal_clue", clueId: "cl_greta" }], ctx);
    expect(a.records[0].description).toContain("révélation");
    const b = await resolveEffects([{ type: "reveal_clue", clueId: "cl_prisonnier" }], ctx);
    expect(b.records[0].description).not.toContain("révélation");
    const c = await resolveEffects([{ type: "reveal_clue", clueId: "cl_prisonnier" }], ctx);
    expect(c.records[0].outcome).toBe("none");
  });

  it("set_state, set_relation, set_flag et reveal_entity écrivent dans le monde", async () => {
    const ctx = ctxWith();
    await resolveEffects(
      [
        { type: "set_state", entityId: "npc1", attribute: "status", value: "dead" },
        { type: "set_relation", fromId: "npc1", toId: "pc1", delta: -2, disposition: "hostile" },
        { type: "set_relation", fromId: "npc1", toId: "pc1", delta: 1 },
        { type: "set_flag", flag: "pont_coupe", value: true },
        { type: "reveal_entity", entityId: "npc1", toUsers: "all_players" },
      ],
      ctx,
    );
    expect(ctx.world.entityAttributes.npc1.status).toBe("dead");
    expect(ctx.world.relations["npc1->pc1"]).toEqual({ value: -1, disposition: "hostile" });
    expect(ctx.world.flags.pont_coupe).toBe(true);
    expect(ctx.world.revealedEntityIds).toEqual(["npc1"]);
  });

  it("trigger_event résout les effets de l’événement, avec une profondeur bornée", async () => {
    const catalog = new Map([
      ["ev_boucle", { name: "Boucle", effects: [{ type: "trigger_event" as const, eventId: "ev_boucle" }] }],
      ["ev_alarme", { name: "Alarme", effects: [{ type: "set_flag" as const, flag: "alarme", value: true }] }],
    ]);
    const ctx = ctxWith(normalizeWorld(null), catalog);
    await resolveEffects([{ type: "trigger_event", eventId: "ev_alarme" }], ctx);
    expect(ctx.world.flags.alarme).toBe(true);
    const loop = await resolveEffects([{ type: "trigger_event", eventId: "ev_boucle" }], ctx);
    expect(loop.records.at(-1)?.description).toContain("trop longue");
  });

  it("ne modifie pas l’état du monde passé en entrée", async () => {
    const world = normalizeWorld(null);
    await resolveEffects([{ type: "set_flag", flag: "x", value: 1 }], ctxWith(world));
    expect(world.flags).toEqual({});
  });
});

describe("sceneTriggers", () => {
  it("propose un déclencheur prêt, puis plus après l’avoir tiré (oneShot)", () => {
    const w = {
      ...normalizeWorld(null),
      currentSceneId: "sc_crypte",
      foundClueIds: ["cl_mira"],
    };
    expect(sceneTriggers(story, w).map((t) => [t.trigger.id, t.ready])).toEqual([["tg_cri", true]]);
    w.firedTriggerIds = ["sc_crypte/tg_cri"];
    expect(sceneTriggers(story, w)[0].ready).toBe(false);
  });
});
