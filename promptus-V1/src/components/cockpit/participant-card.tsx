"use client";

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Heart, Shield, Plus, Minus, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { STANDARD_CONDITIONS } from "@/lib/engine/catalog";
import { cn } from "@/lib/utils";
import type { ParticipantView } from "@/lib/stores/session-store";

interface Props {
  sessionId: string;
  participant: ParticipantView;
}

export function ParticipantCard({ sessionId, participant }: Props) {
  const [busy, setBusy] = useState(false);
  const [hpInput, setHpInput] = useState("1");
  const [condId, setCondId] = useState<string>("");
  const [condRounds, setCondRounds] = useState<string>("");
  const queryClient = useQueryClient();

  const ent = participant.entity;
  const cs = participant.state.currentState;
  const hp = cs.hp ?? 0;
  const hpMax = cs.hpMax ?? hp;
  const ratio = hpMax > 0 ? Math.max(0, Math.min(1, hp / hpMax)) : 0;
  const dead = hp <= 0 && hpMax > 0;

  const barClass =
    ratio > 0.66
      ? "bg-emerald-600"
      : ratio > 0.33
      ? "bg-amber-500"
      : ratio > 0
      ? "bg-red-600"
      : "bg-muted";

  async function patch(body: object) {
    setBusy(true);
    try {
      const res = await fetch(
        `/api/sessions/${sessionId}/state/${participant.state.entityId}`,
        {
          method: "PATCH",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
        },
      );
      if (!res.ok) throw new Error((await res.json()).error?.message ?? "Update failed");
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Update failed");
    } finally {
      setBusy(false);
    }
  }

  function applyCondition() {
    if (!condId) return;
    patch({
      applyCondition: {
        conditionId: condId,
        durationRounds: condRounds ? Number(condRounds) : undefined,
      },
    });
    setCondId("");
    setCondRounds("");
  }

  return (
    <Card data-testid="participant-card" className={cn(dead && "opacity-60")}>
      <CardContent className="pt-4 space-y-3">
        <div className="flex items-start gap-3">
          {ent?.imageUrl ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={ent.imageUrl}
              alt={ent.name}
              className="size-14 rounded object-cover"
            />
          ) : (
            <div className="size-14 rounded bg-muted flex items-center justify-center text-xs uppercase text-muted-foreground">
              {ent?.name.slice(0, 2) ?? "?"}
            </div>
          )}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <h3 className="font-semibold truncate">{ent?.name ?? participant.state.entityId}</h3>
              <Badge variant="outline" className="capitalize text-xs">{ent?.type}</Badge>
            </div>
            <div className="flex items-center gap-3 text-xs text-muted-foreground mt-1">
              <span className="inline-flex items-center gap-1">
                <Heart className="size-3" /> {hp}/{hpMax}
              </span>
              {cs.ac !== undefined ? (
                <span className="inline-flex items-center gap-1">
                  <Shield className="size-3" /> {cs.ac}
                </span>
              ) : null}
            </div>
          </div>
        </div>

        {/* HP bar */}
        <div className="h-2 w-full rounded-full bg-muted overflow-hidden">
          <div
            className={cn("h-full transition-all", barClass)}
            style={{ width: `${ratio * 100}%` }}
            data-testid="hp-bar"
          />
        </div>

        {/* HP controls */}
        <div className="flex items-center gap-1">
          <Button
            type="button"
            size="icon"
            variant="outline"
            onClick={() => patch({ hpDelta: -Number(hpInput || 1) })}
            disabled={busy}
            aria-label="Damage"
          >
            <Minus className="size-4" />
          </Button>
          <Input
            type="number"
            value={hpInput}
            onChange={(e) => setHpInput(e.target.value)}
            className="h-9 w-16 text-center"
          />
          <Button
            type="button"
            size="icon"
            variant="outline"
            onClick={() => patch({ hpDelta: Number(hpInput || 1) })}
            disabled={busy}
            aria-label="Heal"
          >
            <Plus className="size-4" />
          </Button>
          <span className="text-xs text-muted-foreground ml-1">HP</span>
        </div>

        {/* Conditions */}
        {(cs.conditions ?? []).length > 0 ? (
          <div className="flex flex-wrap gap-1">
            {(cs.conditions ?? []).map((c) => (
              <Badge
                key={c.conditionId}
                variant="destructive"
                className="text-xs gap-1"
                data-testid={`condition-${c.conditionId}`}
              >
                {c.conditionId}
                {c.remainingRounds !== undefined ? ` (${c.remainingRounds}r)` : ""}
                <button
                  type="button"
                  onClick={() => patch({ removeCondition: c.conditionId })}
                  className="ml-1 hover:text-destructive-foreground/70"
                  aria-label={`Remove ${c.conditionId}`}
                >
                  <X className="size-3" />
                </button>
              </Badge>
            ))}
          </div>
        ) : null}

        {/* Apply condition */}
        <div className="flex items-center gap-1 pt-1 border-t">
          <Select value={condId} onValueChange={setCondId}>
            <SelectTrigger className="h-8 flex-1 text-xs">
              <SelectValue placeholder="Apply condition" />
            </SelectTrigger>
            <SelectContent>
              {STANDARD_CONDITIONS.map((c) => (
                <SelectItem key={c} value={c}>
                  {c}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            type="number"
            value={condRounds}
            onChange={(e) => setCondRounds(e.target.value)}
            placeholder="rds"
            className="h-8 w-16"
          />
          <Button
            size="sm"
            variant="outline"
            onClick={applyCondition}
            disabled={!condId || busy}
          >
            +
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
