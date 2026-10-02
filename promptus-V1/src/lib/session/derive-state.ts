import type { EntityRow } from "@/lib/db/schema";
import type { EntityState } from "@/lib/engine/types";

/** État de session initial d'un participant, dérivé de sa fiche. */
export function deriveStateFromEntity(row: EntityRow): EntityState {
  const a = row.attributes as Record<string, unknown>;
  return {
    hp: typeof a.hp === "number" ? a.hp : undefined,
    hpMax: typeof a.hpMax === "number" ? a.hpMax : (typeof a.hp === "number" ? a.hp : undefined),
    ac: typeof a.ac === "number" ? a.ac : undefined,
    conditions: [],
    resources: a.spellSlots as EntityState["resources"],
    inventory: Array.isArray(a.inventory) ? (a.inventory as string[]) : [],
    visible: row.visibility !== "mj_only",
  };
}
