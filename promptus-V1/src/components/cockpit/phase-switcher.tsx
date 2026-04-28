"use client";

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Swords, MessageCircle, Map, Bed, Wind } from "lucide-react";
import { cn } from "@/lib/utils";

type Phase = "exploration" | "combat" | "dialogue" | "travel" | "rest";

const PHASES: { id: Phase; label: string; icon: typeof Swords; accent: string }[] = [
  { id: "exploration", label: "Exploration", icon: Map, accent: "from-emerald-500/30 to-emerald-700/30 border-emerald-600" },
  { id: "combat", label: "Combat", icon: Swords, accent: "from-red-500/30 to-red-700/30 border-red-600" },
  { id: "dialogue", label: "Dialogue", icon: MessageCircle, accent: "from-amber-500/30 to-amber-700/30 border-amber-600" },
  { id: "travel", label: "Travel", icon: Wind, accent: "from-sky-500/30 to-sky-700/30 border-sky-600" },
  { id: "rest", label: "Rest", icon: Bed, accent: "from-violet-500/30 to-violet-700/30 border-violet-600" },
];

export function PhaseSwitcher({
  sessionId,
  currentPhase,
}: {
  sessionId: string;
  currentPhase: Phase;
}) {
  const [busy, setBusy] = useState(false);
  const [optimistic, setOptimistic] = useState(currentPhase);
  const queryClient = useQueryClient();

  async function setPhase(phase: Phase) {
    if (phase === optimistic || busy) return;
    setBusy(true);
    setOptimistic(phase);
    try {
      const res = await fetch(`/api/sessions/${sessionId}/phase`, {
        method: "PATCH",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ phase }),
      });
      if (!res.ok) throw new Error("Phase update failed");
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Phase update failed");
      setOptimistic(currentPhase);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex flex-wrap gap-2">
      {PHASES.map((p) => {
        const Icon = p.icon;
        const active = optimistic === p.id;
        return (
          <button
            key={p.id}
            type="button"
            onClick={() => setPhase(p.id)}
            data-active={active ? "true" : "false"}
            data-phase={p.id}
            className={cn(
              "flex flex-col items-center justify-center gap-1 rounded-xl border-2 px-4 py-3 min-w-24 transition-all",
              "bg-gradient-to-br",
              active ? p.accent : "border-border bg-muted/40 hover:bg-muted",
              busy && "opacity-70",
            )}
          >
            <Icon className={cn("size-5", active ? "text-foreground" : "text-muted-foreground")} />
            <span className={cn("text-xs uppercase tracking-wide", active ? "font-semibold" : "text-muted-foreground")}>
              {p.label}
            </span>
          </button>
        );
      })}
    </div>
  );
}
