// Modèle d'histoire d'une campagne — pur TypeScript, sans framework.
// C'est le contrat que le LLM produit en préparation et que le moteur lit en
// partie : une bible, des fronts (menaces qui avancent), un graphe de scènes,
// des révélations atteignables par plusieurs indices, et des cartes à grille.

import type { PhaseId } from "./catalog";
import type { Effect } from "./types";

// ----------------------------------------------------------------------------
// Bible
// ----------------------------------------------------------------------------

export interface CampaignBible {
  /** Le pitch en 2-3 phrases. */
  pitch: string;
  /** Ton et ambiance (« horreur gothique, humour noir »). */
  tone: string;
  themes: string[];
  /** Vérités du monde : ce qui est vrai, que les joueurs le sachent ou non. */
  truths: string[];
  /** Secrets réservés au MJ. */
  secrets: string[];
  /** Accroche présentée aux joueurs en début de campagne. */
  playerHook: string;
  /** Scène d'ouverture. */
  startSceneId?: string;
}

// ----------------------------------------------------------------------------
// Fronts : menaces avec une horloge
// ----------------------------------------------------------------------------

export interface FrontStep {
  label: string;
  description: string;
  /** Effets appliqués quand l'horloge atteint cette étape. */
  effects?: Effect[];
}

export interface Front {
  id: string;
  name: string;
  /** Ce que veut la menace. */
  goal: string;
  description: string;
  /** 3 à 6 étapes ; la dernière est la catastrophe si personne n'agit. */
  steps: FrontStep[];
}

// ----------------------------------------------------------------------------
// Conditions sur l'état du monde (déclencheurs)
// ----------------------------------------------------------------------------

export type SceneStatus = "available" | "visited" | "resolved";

export type WorldCondition =
  | { all: WorldCondition[] }
  | { any: WorldCondition[] }
  | { not: WorldCondition }
  /** Drapeau posé (valeur vraie) ou égal à `equals`. */
  | { flag: string; equals?: unknown }
  | { sceneStatus: { sceneId: string; status: SceneStatus } }
  | { clueFound: string }
  /** Révélation connue : au moins un de ses indices a été trouvé. */
  | { revelationKnown: string }
  | { frontStepAtLeast: { frontId: string; step: number } }
  | { entityAttribute: { entityId: string; attribute: string; equals: unknown } };

// ----------------------------------------------------------------------------
// Scènes : nœuds du graphe
// ----------------------------------------------------------------------------

export interface SceneExit {
  toSceneId: string;
  /** Ce qui mène là (« Suivre les traces vers la forêt »). */
  label: string;
}

export interface SceneTrigger {
  id: string;
  label: string;
  when: WorldCondition;
  effects: Effect[];
  /** Ne se déclenche qu'une fois (défaut : oui). */
  oneShot?: boolean;
}

export interface SceneMedia {
  imagePrompt?: string;
  imageUrl?: string;
  videoPrompt?: string;
  videoUrl?: string;
  /** Musique : lien YouTube choisi par le MJ, ou recherche suggérée. */
  musicUrl?: string;
  musicQuery?: string;
}

export interface Scene {
  id: string;
  title: string;
  /** Pour le MJ : de quoi parle la scène, en 1-2 phrases. */
  summary: string;
  /** Ce que les joueurs peuvent accomplir ici. */
  objective: string;
  /** Texte à lire (ou montrer) aux joueurs en entrant. */
  readAloud: string;
  gmNotes?: string;
  /** Phase de jeu par défaut en entrant dans la scène. */
  phase: PhaseId;
  locationEntityId?: string;
  npcEntityIds: string[];
  monsterEntityIds: string[];
  exits: SceneExit[];
  /** Case de la carte (campagne ou région) où se situe la scène. */
  mapPlacement?: { mapId: string; x: number; y: number };
  /** Carte de combat / lieu ouverte pendant la scène. */
  battleMapId?: string;
  triggers: SceneTrigger[];
  media?: SceneMedia;
}

// ----------------------------------------------------------------------------
// Révélations et indices (règle des trois indices)
// ----------------------------------------------------------------------------

export interface Revelation {
  id: string;
  /** La conclusion que les joueurs doivent pouvoir tirer. */
  statement: string;
  /** critical = nécessaire pour avancer : au moins 3 indices exigés. */
  importance: "critical" | "optional";
}

export interface Clue {
  id: string;
  revelationId: string;
  sceneId: string;
  /** Ce que les joueurs découvrent. */
  text: string;
  /** Comment le découvrir (« Fouiller le bureau »). */
  discovery: string;
  /** Test éventuel, selon le ruleset. */
  check?: { skill?: string; ability?: string; dc?: number };
}

// ----------------------------------------------------------------------------
// Cartes à grille, trois niveaux
// ----------------------------------------------------------------------------

export type MapLevel = "campaign" | "region" | "local";

export interface MapCell {
  x: number;
  y: number;
  terrain?: string;
  /** Case infranchissable (mur, falaise, eau profonde…). */
  blocked?: boolean;
  label?: string;
  sceneId?: string;
  /** Carte de niveau inférieur ouverte depuis cette case. */
  childMapId?: string;
}

export interface MapToken {
  entityId: string;
  x: number;
  y: number;
}

export interface GameMap {
  id: string;
  name: string;
  level: MapLevel;
  /** Hexagones pour campagne et région, carrés pour combat / lieu. */
  grid: { type: "hex" | "square"; cols: number; rows: number };
  backgroundPrompt?: string;
  backgroundUrl?: string;
  /** Cases notables uniquement (les autres sont du terrain par défaut). */
  cells: MapCell[];
  /** Positions de départ (cartes de combat). */
  tokens?: MapToken[];
}

// ----------------------------------------------------------------------------
// Bundle complet
// ----------------------------------------------------------------------------

export interface CampaignStory {
  bible: CampaignBible;
  fronts: Front[];
  scenes: Scene[];
  revelations: Revelation[];
  clues: Clue[];
  maps: GameMap[];
}

export const EMPTY_STORY: CampaignStory = {
  bible: { pitch: "", tone: "", themes: [], truths: [], secrets: [], playerHook: "" },
  fronts: [],
  scenes: [],
  revelations: [],
  clues: [],
  maps: [],
};

export const MAP_LEVEL_LABELS: Record<MapLevel, string> = {
  campaign: "Campagne",
  region: "Région",
  local: "Combat / lieu",
};

export const EXPECTED_GRID: Record<MapLevel, "hex" | "square"> = {
  campaign: "hex",
  region: "hex",
  local: "square",
};
