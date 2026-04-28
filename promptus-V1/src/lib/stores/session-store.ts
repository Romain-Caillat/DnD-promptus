import { create } from "zustand";
import type { EntityState, EntityType, InitiativeEntry } from "@/lib/engine/types";

export interface ParticipantView {
  state: {
    id: string;
    sessionId: string;
    entityId: string;
    currentState: EntityState;
    updatedAt: string | Date;
  };
  entity?: {
    id: string;
    name: string;
    type: EntityType;
    imageUrl: string | null;
    attributes: Record<string, unknown>;
  };
}

export interface SessionView {
  id: string;
  campaignId: string;
  name: string;
  currentPhase: "exploration" | "combat" | "dialogue" | "travel" | "rest";
  initiativeOrder: InitiativeEntry[];
  combatRound: number;
  activeTurnIndex: number;
  startedAt: string | Date;
  endedAt: string | Date | null;
}

interface State {
  session: SessionView | null;
  participants: ParticipantView[];
  set: (s: SessionView, p: ParticipantView[]) => void;
  patchState: (entityId: string, next: ParticipantView["state"]) => void;
}

export const useSessionStore = create<State>((set) => ({
  session: null,
  participants: [],
  set: (s, p) => set({ session: s, participants: p }),
  patchState: (entityId, next) =>
    set((cur) => ({
      participants: cur.participants.map((pv) =>
        pv.state.entityId === entityId ? { ...pv, state: next } : pv,
      ),
    })),
}));
