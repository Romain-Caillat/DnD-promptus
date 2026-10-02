"use client";

import { toast } from "sonner";
import { Dice6, Sword, Sparkles } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { AttackDialog } from "./attack-dialog";
import { CastSpellDialog } from "./cast-spell-dialog";
import type { ParticipantView } from "@/lib/stores/session-store";

type Phase = "exploration" | "combat" | "dialogue" | "travel" | "rest";

interface HotbarProps {
  phase: Phase;
  sessionId: string;
  campaignId: string;
  participants: ParticipantView[];
}

const BUTTON_CLASS = "min-w-32 h-14 flex-col gap-1 shadow-md";

// Seules les actions réellement branchées au moteur sont affichées.
export function Hotbar({ phase, sessionId, campaignId, participants }: HotbarProps) {
  const queryClient = useQueryClient();

  if (phase !== "combat") return null;

  async function rollInitiative() {
    try {
      const res = await fetch(`/api/sessions/${sessionId}/initiative`, { method: "POST" });
      if (!res.ok) throw new Error("Échec du jet d’initiative");
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
      toast.success("Initiative lancée");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Échec du jet d’initiative");
    }
  }

  return (
    <div className="flex flex-wrap gap-2" data-testid="hotbar" data-phase={phase}>
      <Button size="lg" onClick={rollInitiative} className={BUTTON_CLASS}>
        <Dice6 className="size-5" />
        <span className="text-xs">Lancer l’initiative</span>
      </Button>
      <AttackDialog
        sessionId={sessionId}
        participants={participants}
        trigger={
          <Button size="lg" className={BUTTON_CLASS}>
            <Sword className="size-5" />
            <span className="text-xs">Attaquer</span>
          </Button>
        }
      />
      <CastSpellDialog
        sessionId={sessionId}
        campaignId={campaignId}
        participants={participants}
        trigger={
          <Button size="lg" className={BUTTON_CLASS}>
            <Sparkles className="size-5" />
            <span className="text-xs">Lancer un sort</span>
          </Button>
        }
      />
    </div>
  );
}
