// Vue joueur : tout ce qu'un joueur a le droit de voir, et rien d'autre.
// Fonction pure (testée contre les fuites) : pas de notes MJ, pas de cases
// non révélées, pas de nom d'adversaire caché, pas de secrets.

import type { PhaseId } from "@/lib/engine/catalog";
import { attackGeometry, creatureAttacks } from "@/lib/engine/combat";
import { cellKey, speedInCells, type DiagonalRule } from "@/lib/engine/grid";
import { abilityModifier, damageTypeLabel, rulesetConditionLabel, skillLabel, type Ruleset } from "@/lib/engine/ruleset";
import type { CampaignStory, GameMap, MapCell, MapToken } from "@/lib/engine/story";
import type { MusicState } from "@/lib/media/youtube";
import type { EntityState, EntityType, InitiativeEntry } from "@/lib/engine/types";
import { defaultMapId, mapTokens, revealedCells, type WorldState } from "@/lib/engine/world";

export interface ProjectionEntity {
  id: string;
  name: string;
  type: EntityType;
  description: string | null;
  imageUrl: string | null;
  visibility: string;
  attributes: Record<string, unknown>;
}

export interface ProjectionInput {
  session: {
    id: string;
    name: string;
    currentPhase: PhaseId;
    combatRound: number;
    activeTurnIndex: number;
    initiativeOrder: InitiativeEntry[];
  };
  campaignName: string;
  story: CampaignStory;
  world: WorldState;
  ruleset: Ruleset;
  /** Toutes les fiches de la campagne. */
  entities: ProjectionEntity[];
  /** États de session des participants. */
  states: { entityId: string; currentState: EntityState }[];
  player: { id: string; name: string; characterEntityId: string | null };
  /** Personnages déjà pris par d'autres joueurs, et par qui. */
  takenBy: Record<string, string>;
  timeline: { id: string; description: string; createdAt: string; kind?: "event" | "narration" | "npc" }[];
  requests: { id: string; label: string; status: string; result: string | null; createdAt: string }[];
  /** Heure serveur (ms), pour synchroniser la musique. */
  now?: number;
}

export interface PlayerMap {
  id: string;
  name: string;
  level: GameMap["level"];
  grid: GameMap["grid"];
  backgroundUrl?: string;
  /** Uniquement les cases révélées. */
  cells: MapCell[];
  tokens: (MapToken & { label: string; kind: "pc" | "enemy" | "npc" | "party"; mine: boolean; down: boolean })[];
  revealed: string[];
}

export interface PlayerView {
  session: { id: string; name: string; phase: PhaseId; combatRound: number; campaignName: string };
  me: { playerId: string; name: string; characterId: string | null };
  character: {
    id: string;
    name: string;
    description: string | null;
    imageUrl: string | null;
    hp: number | null;
    hpMax: number | null;
    ac: number | null;
    conditions: { id: string; label: string; remainingRounds?: number }[];
    abilities: { id: string; abbr: string; label: string; score: number; modifier: number }[];
    resources: { id: string; current: number; max: number }[];
    inventory: string[];
  } | null;
  party: { id: string; name: string; hp: number | null; hpMax: number | null; player: string | null }[];
  initiative: { name: string; isPlayer: boolean; active: boolean; mine: boolean }[];
  isMyTurn: boolean;
  scene: { title: string; readAloud: string; phase: PhaseId; imageUrl: string | null; videoUrl: string | null } | null;
  map: PlayerMap | null;
  movement: { budgetCells: number; usedCells: number; diagonal: DiagonalRule; allowed: boolean; reason: string | null } | null;
  actions: { id: string; label: string; kind: string; detail: string | null; description: string | null }[];
  /** Attaques du personnage (portées en mètres). */
  attacks: { id: string; name: string; bonus: number; damage: string; damageType: string; rangeMeters: number; longRangeMeters: number | null }[];
  /** Cibles visibles sur la carte, et pour chaque attaque si elle est possible. */
  targets: {
    entityId: string;
    label: string;
    kind: "pc" | "enemy" | "npc";
    down: boolean;
    distanceMeters: number;
    byAttack: Record<string, { possible: boolean; longRange: boolean; reason: string | null }>;
  }[];
  spotlight: { name: string; description: string | null; imageUrl: string | null } | null;
  /** Musique d'ambiance en cours (YouTube) et heure serveur pour la caler. */
  music: MusicState | null;
  serverTime: number;
  /** Dernière narration ou réplique envoyée par le MJ. */
  narration: { id: string; text: string } | null;
  timeline: ProjectionInput["timeline"];
  requests: ProjectionInput["requests"];
}

/** Nom visible d'une fiche : vrai nom si publique ou révélée, sinon générique. */
function visibleName(e: ProjectionEntity | undefined, world: WorldState): string {
  if (!e) return "Inconnu";
  if (e.type === "character" || e.visibility === "public" || world.revealedEntityIds.includes(e.id)) return e.name;
  return e.type === "npc" ? "Inconnu" : "Adversaire";
}

export function turnKey(round: number, turnIndex: number): string {
  return `${round}:${turnIndex}`;
}

export function projectPlayerView(input: ProjectionInput): PlayerView {
  const { session, story, world, ruleset, player } = input;
  const byId = new Map(input.entities.map((e) => [e.id, e]));
  const stateById = new Map(input.states.map((s) => [s.entityId, s.currentState]));
  const myId = player.characterEntityId;

  // Adversaires et PNJ non révélés : nom générique, numéroté s'il y en a
  // plusieurs (ordre stable : par identifiant).
  const mapForNames = story.maps.find((m) => m.id === defaultMapId(story, world));
  const revealedForNames = mapForNames ? revealedCells(mapForNames, world) : new Set<string>();
  const seenIds = new Set([
    ...session.initiativeOrder.map((e) => e.entityId),
    ...(mapForNames ? mapTokens(mapForNames, world).filter((t) => revealedForNames.has(cellKey(t.x, t.y))).map((t) => t.entityId) : []),
  ]);
  const generic = new Map<string, string>();
  for (const kind of ["Adversaire", "Inconnu"]) {
    const ids = [...seenIds]
      .filter((id) => {
        const e = byId.get(id);
        return e && visibleName(e, world) === kind;
      })
      .sort();
    ids.forEach((id, i) => generic.set(id, ids.length > 1 ? `${kind} ${i + 1}` : kind));
  }
  const nameOf = (e: ProjectionEntity | undefined) => (e && generic.get(e.id)) ?? visibleName(e, world);
  const me = myId ? byId.get(myId) : undefined;
  const myState = myId ? stateById.get(myId) : undefined;

  const inCombat = session.currentPhase === "combat" && session.combatRound > 0 && session.initiativeOrder.length > 0;
  const active = inCombat ? session.initiativeOrder[session.activeTurnIndex] : undefined;
  const isMyTurn = !!active && active.entityId === myId;

  // --- Personnage ---------------------------------------------------------
  const character =
    me && myId
      ? {
          id: me.id,
          name: me.name,
          description: me.description,
          imageUrl: me.imageUrl,
          hp: myState?.hp ?? (me.attributes.hp as number | undefined) ?? null,
          hpMax: myState?.hpMax ?? (me.attributes.hpMax as number | undefined) ?? null,
          ac: myState?.ac ?? (me.attributes.ac as number | undefined) ?? null,
          conditions: (myState?.conditions ?? []).map((c) => ({
            id: c.conditionId,
            label: rulesetConditionLabel(ruleset, c.conditionId),
            remainingRounds: c.remainingRounds,
          })),
          abilities: ruleset.abilities.map((a) => {
            const score = ((me.attributes.abilityScores as Record<string, number> | undefined) ?? {})[a.id] ?? 10;
            return { id: a.id, abbr: a.abbr, label: a.label, score, modifier: abilityModifier(ruleset, score) };
          }),
          resources: Object.entries(myState?.resources ?? {}).map(([id, r]) => ({ id, current: r.current, max: r.max })),
          inventory: (myState?.inventory ?? []).map((id) => byId.get(id)?.name ?? id),
        }
      : null;

  // --- Groupe et initiative ----------------------------------------------
  const party = input.states
    .map((s) => byId.get(s.entityId))
    .filter((e): e is ProjectionEntity => !!e && e.type === "character")
    .map((e) => ({
      id: e.id,
      name: e.name,
      hp: stateById.get(e.id)?.hp ?? null,
      hpMax: stateById.get(e.id)?.hpMax ?? null,
      player: input.takenBy[e.id] ?? null,
    }));
  const initiative = inCombat
    ? session.initiativeOrder.map((entry, i) => ({
        name: nameOf(byId.get(entry.entityId)),
        isPlayer: entry.isPlayer,
        active: i === session.activeTurnIndex,
        mine: entry.entityId === myId,
      }))
    : [];

  // --- Scène ---------------------------------------------------------------
  const sceneRow = story.scenes.find((s) => s.id === world.currentSceneId);
  const scene = sceneRow
    ? {
        title: sceneRow.title,
        readAloud: sceneRow.readAloud,
        phase: sceneRow.phase,
        imageUrl: sceneRow.media?.imageUrl ?? null,
        videoUrl: sceneRow.media?.videoUrl ?? null,
      }
    : null;

  // --- Carte (cases révélées uniquement) ----------------------------------
  const mapId = defaultMapId(story, world);
  const mapRow = story.maps.find((m) => m.id === mapId);
  let map: PlayerMap | null = null;
  if (mapRow) {
    const revealed = revealedCells(mapRow, world);
    map = {
      id: mapRow.id,
      name: mapRow.name,
      level: mapRow.level,
      grid: mapRow.grid,
      backgroundUrl: mapRow.backgroundUrl,
      cells: mapRow.cells
        .filter((c) => revealed.has(cellKey(c.x, c.y)))
        .map((c) => ({ x: c.x, y: c.y, terrain: c.terrain, blocked: c.blocked, label: c.label })),
      tokens: mapTokens(mapRow, world)
        .filter((t) => revealed.has(cellKey(t.x, t.y)))
        .map((t) => {
          const e = byId.get(t.entityId);
          const kind = t.entityId === "party" ? "party" : e?.type === "character" ? "pc" : e?.type === "npc" ? "npc" : "enemy";
          const name = t.entityId === "party" ? "Le groupe" : nameOf(e);
          const hp = stateById.get(t.entityId)?.hp;
          return { ...t, label: name, kind, mine: t.entityId === myId, down: typeof hp === "number" && hp <= 0 };
        }),
      revealed: [...revealed],
    };
  }

  // --- Déplacement autorisé ------------------------------------------------
  let movement: PlayerView["movement"] = null;
  if (map && mapRow?.level === "local" && myId && map.tokens.some((t) => t.mine)) {
    const budget = speedInCells(me?.attributes ?? {}, ruleset.movement.local.cellMeters, ruleset.movement.local.defaultSpeedCells);
    const key = turnKey(session.combatRound, session.activeTurnIndex);
    const used = inCombat && world.turnMovement?.turnKey === key ? (world.turnMovement.used[myId] ?? 0) : 0;
    const reason = inCombat && !isMyTurn ? "Ce n’est pas votre tour" : used >= budget ? "Déplacement épuisé pour ce tour" : null;
    movement = { budgetCells: budget, usedCells: used, diagonal: ruleset.movement.local.diagonal, allowed: !reason, reason };
  }

  // --- Actions du ruleset pour la phase -----------------------------------
  const actions = ruleset.actions
    .filter((a) => a.phases.includes(session.currentPhase))
    .map((a) => ({
      id: a.id,
      label: a.label,
      kind: a.kind,
      detail: a.skill ? skillLabel(ruleset, a.skill) : a.ability ? (ruleset.abilities.find((x) => x.id === a.ability)?.label ?? a.ability) : null,
      description: a.description ?? null,
    }));

  // --- Attaques et cibles ------------------------------------------------
  const attackList = me ? creatureAttacks(me.attributes, ruleset) : [];
  const attacks = attackList.map((a) => ({
    id: a.id,
    name: a.name,
    bonus: a.bonus,
    damage: a.damage,
    damageType: damageTypeLabel(ruleset, a.damageType),
    rangeMeters: a.rangeMeters,
    longRangeMeters: a.longRangeMeters ?? null,
  }));
  const myToken = map?.tokens.find((t) => t.mine);
  const targets: PlayerView["targets"] =
    map && mapRow && myToken && mapRow.level === "local"
      ? map.tokens
          .filter((t) => !t.mine && t.kind !== "party")
          .map((t) => {
            const byAttack: PlayerView["targets"][number]["byAttack"] = {};
            let meters = 0;
            for (const a of attackList) {
              const g = attackGeometry({
                map: mapRow,
                from: myToken,
                to: t,
                attack: a,
                cellMeters: ruleset.movement.local.cellMeters,
                diagonal: ruleset.movement.local.diagonal,
              });
              meters = g.distanceMeters;
              byAttack[a.id] = { possible: !g.reason, longRange: g.longRange, reason: g.reason };
            }
            return { entityId: t.entityId, label: t.label, kind: t.kind as "pc" | "enemy" | "npc", down: t.down, distanceMeters: meters, byAttack };
          })
      : [];

  // Les noms des fiches cachées n'apparaissent pas dans les textes publics.
  const hidden = input.entities
    .filter((e) => visibleName(e, world) !== e.name && e.name.length > 2)
    .sort((a, b) => b.name.length - a.name.length);
  // Les PV des adversaires et PNJ restent secrets (« PV 10→4 »).
  const escape = (x: string) => x.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const nonPc = [...new Set(input.entities.filter((e) => e.type !== "character").map((e) => nameOf(e)))];
  const hpPattern = nonPc.length ? new RegExp(`(${nonPc.map(escape).join("|")}) \\(PV -?\\d+→-?\\d+\\)`, "g") : null;
  const mask = (text: string) => {
    const masked = hidden.reduce((t, e) => t.split(e.name).join(nameOf(e)), text);
    return hpPattern ? masked.replace(hpPattern, "$1") : masked;
  };

  const spot = world.spotlightEntityId ? byId.get(world.spotlightEntityId) : undefined;

  return {
    session: { id: session.id, name: session.name, phase: session.currentPhase, combatRound: session.combatRound, campaignName: input.campaignName },
    me: { playerId: player.id, name: player.name, characterId: myId },
    character,
    party,
    initiative,
    isMyTurn,
    scene,
    map,
    movement,
    actions,
    attacks,
    targets,
    spotlight: spot ? { name: spot.name, description: spot.description, imageUrl: spot.imageUrl } : null,
    music: world.music ?? null,
    serverTime: input.now ?? Date.now(),
    narration: (() => {
      const n = input.timeline.find((t) => t.kind === "narration" || t.kind === "npc");
      return n ? { id: n.id, text: mask(n.description) } : null;
    })(),
    timeline: input.timeline.map((t) => ({ ...t, description: mask(t.description) })),
    requests: input.requests.map((r) => ({ ...r, result: r.result ? mask(r.result) : null })),
  };
}
