"use client";

import { toast } from "sonner";
import {
  Dice6,
  Sword,
  Sparkles,
  ShieldCheck,
  Footprints,
  Eye,
  Search,
  EyeOff,
  Map,
  MessageSquare,
  Brain,
  StickyNote,
  Coffee,
  HeartPulse,
  Wind,
  Hand,
} from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { AttackDialog } from "./attack-dialog";
import { CastSpellDialog } from "./cast-spell-dialog";
import type { ParticipantView } from "@/lib/stores/session-store";

type Phase = "exploration" | "combat" | "dialogue" | "travel" | "rest";

interface Action {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  hotkey?: string;
  onClick?: () => void;
  primary?: boolean;
}

const PHASE_ACTIONS: Record<Phase, Action[]> = {
  combat: [
    { label: "Roll initiative", icon: Dice6, primary: true },
    { label: "Attack", icon: Sword, primary: true },
    { label: "Cast spell", icon: Sparkles, primary: true },
    { label: "Defensive action", icon: ShieldCheck },
    { label: "Move / dash", icon: Footprints },
  ],
  exploration: [
    { label: "Perception", icon: Eye, primary: true },
    { label: "Investigation", icon: Search, primary: true },
    { label: "Stealth", icon: EyeOff, primary: true },
    { label: "Move to location", icon: Map },
    { label: "Survival", icon: Hand },
  ],
  dialogue: [
    { label: "Persuasion", icon: MessageSquare, primary: true },
    { label: "Deception", icon: Brain, primary: true },
    { label: "Intimidation", icon: HeartPulse, primary: true },
    { label: "Insight", icon: Eye },
    { label: "Note", icon: StickyNote },
  ],
  travel: [
    { label: "Travel pace", icon: Footprints, primary: true },
    { label: "Random encounter", icon: Dice6 },
    { label: "Map move", icon: Map, primary: true },
    { label: "Survival check", icon: Hand },
    { label: "Weather", icon: Wind },
  ],
  rest: [
    { label: "Short rest", icon: Coffee, primary: true },
    { label: "Long rest", icon: Coffee, primary: true },
    { label: "Heal", icon: HeartPulse },
    { label: "Restore resources", icon: Sparkles },
    { label: "Note", icon: StickyNote },
  ],
};

interface HotbarProps {
  phase: Phase;
  sessionId: string;
  campaignId: string;
  participants: ParticipantView[];
}

export function Hotbar({ phase, sessionId, campaignId, participants }: HotbarProps) {
  const actions = PHASE_ACTIONS[phase] ?? [];
  const queryClient = useQueryClient();

  function placeholder(label: string) {
    toast.info(`${label}: not yet wired (V1)`);
  }

  async function rollInitiative() {
    try {
      const res = await fetch(`/api/sessions/${sessionId}/initiative`, { method: "POST" });
      if (!res.ok) throw new Error("Initiative roll failed");
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
      toast.success("Initiative rolled");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Initiative failed");
    }
  }

  return (
    <div className="flex flex-wrap gap-2" data-testid="hotbar" data-phase={phase}>
      {actions.map((a) => {
        const Icon = a.icon;
        const buttonNode = (
          <Button
            key={a.label}
            variant={a.primary ? "default" : "outline"}
            size="lg"
            className={cn("min-w-32 h-14 flex-col gap-1", a.primary && "shadow-md")}
          >
            <Icon className="size-5" />
            <span className="text-xs">{a.label}</span>
          </Button>
        );

        // Combat: wire Attack and Cast spell to dialogs
        if (phase === "combat" && a.label === "Attack") {
          return (
            <AttackDialog
              key={a.label}
              sessionId={sessionId}
              participants={participants}
              trigger={buttonNode}
            />
          );
        }
        if (phase === "combat" && a.label === "Cast spell") {
          return (
            <CastSpellDialog
              key={a.label}
              sessionId={sessionId}
              campaignId={campaignId}
              participants={participants}
              trigger={buttonNode}
            />
          );
        }
        if (phase === "combat" && a.label === "Roll initiative") {
          return (
            <Button
              key={a.label}
              variant={a.primary ? "default" : "outline"}
              size="lg"
              onClick={rollInitiative}
              className={cn("min-w-32 h-14 flex-col gap-1", a.primary && "shadow-md")}
            >
              <Icon className="size-5" />
              <span className="text-xs">{a.label}</span>
            </Button>
          );
        }

        return (
          <Button
            key={a.label}
            variant={a.primary ? "default" : "outline"}
            size="lg"
            onClick={() => (a.onClick ? a.onClick() : placeholder(a.label))}
            className={cn("min-w-32 h-14 flex-col gap-1", a.primary && "shadow-md")}
          >
            <Icon className="size-5" />
            <span className="text-xs">{a.label}</span>
          </Button>
        );
      })}
    </div>
  );
}
