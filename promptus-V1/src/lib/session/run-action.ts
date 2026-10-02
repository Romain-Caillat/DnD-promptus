import "server-only";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db/client";
import { lockCampaign } from "@/lib/db/lock";
import { campaigns, entities, sessions, sessionState, sessionTimeline } from "@/lib/db/schema";
import { badRequest, notFound } from "@/lib/api/errors";
import { generateId } from "@/lib/api/ids";
import { attackGeometry, creatureAttacks, tokenPositions } from "@/lib/engine/combat";
import {
  newContext,
  resolveAttack,
  resolveEffects,
  type CatalogEntry,
  type EntityRef,
} from "@/lib/engine/resolver";
import { DND5E_RULESET } from "@/lib/engine/ruleset";
import { EMPTY_STORY } from "@/lib/engine/story";
import { defaultMapId, normalizeWorld, triggerKey, type WorldState } from "@/lib/engine/world";
import type { Effect, EntityState, ResolutionRecord } from "@/lib/engine/types";
import type { ActionInput } from "@/lib/validation/action-schemas";
import { notifySession } from "@/lib/realtime/notify";

/** Effets qui relèvent de la cuisine du MJ : invisibles dans le journal des joueurs. */
const GM_ONLY_EFFECTS = new Set<Effect["type"]>([
  "advance_front",
  "set_flag",
  "set_state",
  "set_relation",
  "set_scene_status",
  "trigger_event",
  "play_ambience",
  "play_music",
  "play_sound",
  "display_image",
]);

function isPublicRecord(r: ResolutionRecord, isTriggerHeader: boolean): boolean {
  return !isTriggerHeader && !GM_ONLY_EFFECTS.has(r.effect.type);
}

/**
 * Résout une action de jeu dans une session : effets, attaques (avec portée
 * et ligne de vue sur la carte affichée), déclencheurs. Persiste états,
 * monde et journal, puis notifie les clients.
 */
export async function runAction(
  sessionId: string,
  action: ActionInput,
): Promise<{ records: ResolutionRecord[]; world: WorldState }> {
  const [session] = await db.select().from(sessions).where(eq(sessions.id, sessionId));
  if (!session) notFound("session", sessionId);
  const ents = await db.select().from(entities).where(eq(entities.campaignId, session.campaignId));

  // Tout se fait sous verrou : les actions simultanées (MJ, joueurs) passent
  // l'une après l'autre sans écraser l'état du monde ni les PV.
  const result = await db.transaction(async (tx) => {
    const campaign = await lockCampaign(tx, session.campaignId);

    const states = await tx.select().from(sessionState).where(eq(sessionState.sessionId, sessionId));
    const entById = new Map(ents.map((e) => [e.id, e]));
    const stateRowById = new Map(states.map((s) => [s.entityId, s]));

    const refs: EntityRef[] = states.map((s) => {
      const ent = entById.get(s.entityId);
      return {
        id: s.entityId,
        name: ent?.name ?? s.entityId,
        attributes: (ent?.attributes ?? {}) as Record<string, unknown>,
        state: s.currentState as EntityState,
      };
    });

    const casterId =
      action.kind === "attack" ? action.attackerId : action.kind === "fire_trigger" ? undefined : action.casterId;
    const ruleset = campaign.ruleset ?? DND5E_RULESET;
    const story = campaign.story ?? EMPTY_STORY;
    const worldBefore = normalizeWorld(campaign.worldState);
    const ctx = newContext({
      ruleset,
      story,
      world: worldBefore,
      catalog: new Map<string, CatalogEntry>(ents.map((e) => [e.id, { name: e.name, effects: e.effects as Effect[] }])),
      caster: casterId ? refs.find((r) => r.id === casterId) : undefined,
      entities: refs,
      initiativeOrder: session.initiativeOrder,
      explicitTargets: "targetIds" in action && action.targetIds?.length ? action.targetIds : undefined,
    });

    let records: ResolutionRecord[] = [];
    switch (action.kind) {
      case "attack": {
        const attacker = entById.get(action.attackerId);
        if (!attacker || !stateRowById.has(action.attackerId)) badRequest("L’attaquant ne participe pas à la session");
        for (const t of action.targetIds) if (!stateRowById.has(t)) badRequest("Une cible ne participe pas à la session");
        const weapon = action.attackId
          ? creatureAttacks(attacker.attributes, ruleset).find((a) => a.id === action.attackId)
          : undefined;
        if (action.attackId && !weapon) badRequest("Attaque inconnue pour cette fiche");
        const bonus = weapon?.bonus ?? action.attackBonus;
        const damage = weapon?.damage ?? action.damageNotation;
        const damageType = weapon?.damageType ?? action.damageType;
        if (bonus === undefined || !damage || !damageType) badRequest("Précisez le bonus, les dégâts et leur type");

        // Sur une carte de combat affichée, la portée et la ligne de vue comptent.
        const map = story.maps.find((m) => m.id === defaultMapId(story, worldBefore));
        for (const targetId of action.targetIds) {
          const pos = map?.level === "local" ? tokenPositions(map, worldBefore, action.attackerId, targetId) : null;
          const geo =
            pos && map
              ? attackGeometry({
                  map,
                  ...pos,
                  attack: weapon ?? { rangeMeters: Infinity },
                  cellMeters: ruleset.movement.local.cellMeters,
                  diagonal: ruleset.movement.local.diagonal,
                })
              : null;
          if (geo?.reason && weapon && !action.force) {
            badRequest(`${entById.get(targetId)?.name ?? "Cible"} : ${geo.reason}`);
          }
          records.push(
            ...resolveAttack(
              {
                attackerId: action.attackerId,
                targetIds: [targetId],
                attackBonus: bonus,
                damageNotation: damage,
                damageType,
                meleeWithin5ft: geo ? geo.melee : action.meleeWithin5ft,
                attackName: weapon?.name,
                disadvantage: !!geo?.longRange && !action.force,
              },
              ctx,
            ),
          );
        }
        break;
      }
      case "cast_spell": {
        const spell = entById.get(action.spellEntityId);
        if (!spell || spell.type !== "spell") badRequest(`Sort ${action.spellEntityId} introuvable`);
        records = (await resolveEffects(spell.effects as Effect[], ctx)).records;
        break;
      }
      case "apply_entity_effects": {
        const ent = entById.get(action.entityId);
        if (!ent) badRequest(`Fiche ${action.entityId} introuvable`);
        records = (await resolveEffects(ent.effects as Effect[], ctx)).records;
        break;
      }
      case "raw_effects":
        records = (await resolveEffects(action.effects, ctx)).records;
        break;
      case "fire_trigger": {
        const scene = story.scenes.find((s) => s.id === action.sceneId);
        const trigger = scene?.triggers.find((t) => t.id === action.triggerId);
        if (!scene || !trigger) badRequest(`Déclencheur ${action.sceneId}/${action.triggerId} introuvable`);
        const key = triggerKey(scene.id, trigger.id);
        if (!ctx.world.firedTriggerIds.includes(key)) ctx.world.firedTriggerIds.push(key);
        const sub = await resolveEffects(trigger.effects, ctx);
        records = [
          {
            effect: { type: "display_text", text: trigger.label },
            rolls: [],
            outcome: "success",
            applied: [],
            description: `⚡ Déclencheur : ${trigger.label}`,
            timestamp: new Date().toISOString(),
          },
          ...sub.records,
        ];
        break;
      }
    }

    // En entrant dans une scène, la session passe dans la phase de la scène.
    const entered = [...records].reverse().find((r) => r.effect.type === "enter_scene");
    const enteredScene =
      entered?.effect.type === "enter_scene"
        ? story.scenes.find((s) => s.id === (entered.effect as { sceneId: string }).sceneId)
        : undefined;

    for (const [entityId, nextState] of ctx.states.entries()) {
      const row = stateRowById.get(entityId);
      if (!row) continue;
      if (JSON.stringify(row.currentState) !== JSON.stringify(nextState)) {
        await tx.update(sessionState).set({ currentState: nextState, updatedAt: new Date() }).where(eq(sessionState.id, row.id));
      }
    }
    if (JSON.stringify(ctx.world) !== JSON.stringify(worldBefore)) {
      await tx.update(campaigns).set({ worldState: ctx.world, updatedAt: new Date() }).where(eq(campaigns.id, campaign.id));
    }
    if (enteredScene && enteredScene.phase !== session.currentPhase) {
      await tx.update(sessions).set({ currentPhase: enteredScene.phase }).where(eq(sessions.id, sessionId));
    }
    if (records.length) {
      // Horodatages croissants : le journal est trié par date de création.
      const now = Date.now();
      await tx.insert(sessionTimeline).values(
        records.map((r, i) => ({
          id: generateId("tl"),
          sessionId,
          round: session.combatRound,
          description: r.description,
          resolutionRecord: r,
          isPublic: isPublicRecord(r, action.kind === "fire_trigger" && i === 0),
          createdAt: new Date(now + i),
        })),
      );
    }
    return { records, world: ctx.world };
  });

  await notifySession(sessionId, ["session", "story", "timeline"]);
  return result;
}
