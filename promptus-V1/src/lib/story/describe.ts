// Descriptions lisibles (français) des éléments d'histoire, pour l'UI du MJ.

import type { CampaignStory, WorldCondition } from "@/lib/engine/story";

const STATUS_LABELS = { available: "disponible", visited: "visitée", resolved: "résolue" } as const;

export function describeCondition(
  c: WorldCondition,
  story: CampaignStory,
  entityName: (id: string) => string,
): string {
  const scene = (id: string) => story.scenes.find((s) => s.id === id)?.title ?? id;
  if ("all" in c) return c.all.map((x) => describeCondition(x, story, entityName)).join(" et ");
  if ("any" in c) return `(${c.any.map((x) => describeCondition(x, story, entityName)).join(" ou ")})`;
  if ("not" in c) return `non ${describeCondition(c.not, story, entityName)}`;
  if ("flag" in c) return c.equals === undefined ? `drapeau « ${c.flag} »` : `« ${c.flag} » = ${JSON.stringify(c.equals)}`;
  if ("sceneStatus" in c) return `« ${scene(c.sceneStatus.sceneId)} » ${STATUS_LABELS[c.sceneStatus.status]}`;
  if ("clueFound" in c) {
    const clue = story.clues.find((x) => x.id === c.clueFound);
    return `indice trouvé : ${clue ? `« ${clue.discovery || clue.text} »` : c.clueFound}`;
  }
  if ("revelationKnown" in c) {
    const r = story.revelations.find((x) => x.id === c.revelationKnown);
    return `révélation connue : « ${r?.statement ?? c.revelationKnown} »`;
  }
  if ("frontStepAtLeast" in c) {
    const f = story.fronts.find((x) => x.id === c.frontStepAtLeast.frontId);
    const step = f?.steps[c.frontStepAtLeast.step];
    return `« ${f?.name ?? c.frontStepAtLeast.frontId} » atteint « ${step?.label ?? `étape ${c.frontStepAtLeast.step + 1}`} »`;
  }
  return `${entityName(c.entityAttribute.entityId)}.${c.entityAttribute.attribute} = ${JSON.stringify(c.entityAttribute.equals)}`;
}
