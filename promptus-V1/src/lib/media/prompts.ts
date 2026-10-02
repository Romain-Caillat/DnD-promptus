// Prompts des médias d'une campagne, dérivés du scénario et du guide de style.
// Fonctions pures : le MJ peut toujours remplacer le prompt proposé.

import { buildEntityImagePrompt } from "@/lib/ai/prompt-template";
import { gridPixelSize } from "@/lib/engine/grid";
import type { CampaignBible, GameMap, Scene } from "@/lib/engine/story";
import type { StyleGuide } from "@/lib/engine/types";

export type MediaTarget =
  | { type: "entity"; id: string }
  | { type: "scene_image"; id: string }
  | { type: "scene_video"; id: string }
  | { type: "map"; id: string };

export const MEDIA_TARGET_LABELS: Record<MediaTarget["type"], string> = {
  entity: "Image de fiche",
  scene_image: "Image de scène",
  scene_video: "Vidéo d’intro",
  map: "Fond de carte",
};

export function targetKey(t: MediaTarget): string {
  return `${t.type}:${t.id}`;
}

export function isVideo(t: MediaTarget): boolean {
  return t.type === "scene_video";
}

function style(guide: StyleGuide, bible?: CampaignBible): string {
  return [guide.promptPrefix, guide.artStyle, guide.mood ?? bible?.tone, guide.palette && `palette : ${guide.palette}`, guide.promptSuffix]
    .filter(Boolean)
    .join(", ");
}

export function entityImagePrompt(
  e: { name: string; description: string | null; tags: string[]; type: string; attributes: Record<string, unknown> },
  guide: StyleGuide,
): string {
  if (typeof e.attributes.imagePrompt === "string" && e.attributes.imagePrompt.trim()) return e.attributes.imagePrompt.trim();
  return buildEntityImagePrompt({ name: e.name, description: e.description, tags: e.tags, type: e.type, styleGuide: guide });
}

export function sceneImagePrompt(scene: Scene, guide: StyleGuide, bible?: CampaignBible): string {
  if (scene.media?.imagePrompt?.trim()) return scene.media.imagePrompt.trim();
  return [style(guide, bible), `Illustration de scène de jeu de rôle : ${scene.title}. ${scene.readAloud}`, "sans texte"]
    .filter(Boolean)
    .join(". ");
}

export function sceneVideoPrompt(scene: Scene, guide: StyleGuide, bible?: CampaignBible): string {
  if (scene.media?.videoPrompt?.trim()) return scene.media.videoPrompt.trim();
  return [style(guide, bible), `Courte vidéo d’introduction, plan cinématique lent : ${scene.title}. ${scene.readAloud}`, "sans texte ni dialogue"]
    .filter(Boolean)
    .join(". ");
}

export function mapBackgroundPrompt(map: GameMap, guide: StyleGuide, bible?: CampaignBible): string {
  const kind =
    map.level === "campaign" ? "carte du monde dessinée à la main" : map.level === "region" ? "carte régionale illustrée" : "plan de combat vu du dessus (battlemap)";
  const subject = map.backgroundPrompt?.trim() || map.name;
  return [style(guide, bible), `${kind}, vue de dessus : ${subject}`, "sans texte, sans grille, sans personnages"].filter(Boolean).join(". ");
}

const RATIOS: [string, number][] = [
  ["1:1", 1],
  ["4:3", 4 / 3],
  ["3:4", 3 / 4],
  ["3:2", 3 / 2],
  ["2:3", 2 / 3],
  ["16:9", 16 / 9],
  ["9:16", 9 / 16],
  ["21:9", 21 / 9],
];

/** Format d'image le plus proche d'un rapport largeur/hauteur. */
export function closestRatio(ratio: number): string {
  return RATIOS.reduce((best, r) => (Math.abs(Math.log(r[1] / ratio)) < Math.abs(Math.log(best[1] / ratio)) ? r : best))[0];
}

export function mapAspectRatio(map: GameMap): string {
  const { width, height } = gridPixelSize(map.grid.type, map.grid.cols, map.grid.rows, map.grid.type === "hex" ? 22 : 30);
  return closestRatio(width / height);
}
