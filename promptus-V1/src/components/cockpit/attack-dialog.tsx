"use client";

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { OUTCOME_LABELS } from "@/lib/engine/catalog";
import { useRuleset } from "@/components/providers/ruleset-provider";
import type { ParticipantView } from "@/lib/stores/session-store";
import type { ResolutionRecord } from "@/lib/engine/types";

interface Props {
  sessionId: string;
  participants: ParticipantView[];
  trigger: React.ReactNode;
}

export function AttackDialog({ sessionId, participants, trigger }: Props) {
  const ruleset = useRuleset();
  const [open, setOpen] = useState(false);
  const [attackerId, setAttackerId] = useState<string>("");
  const [targetIds, setTargetIds] = useState<Set<string>>(new Set());
  const [attackBonus, setAttackBonus] = useState("3");
  const [damageNotation, setDamageNotation] = useState("1d8+3");
  const [damageTypeChoice, setDamageType] = useState<string>("");
  // Le type par défaut est le premier du ruleset (tranchant en 5e).
  const damageType = damageTypeChoice || ruleset.damageTypes[0]?.id || "";
  const [meleeWithin5ft, setMeleeWithin5ft] = useState(true);
  const [busy, setBusy] = useState(false);
  const [results, setResults] = useState<ResolutionRecord[] | null>(null);
  const queryClient = useQueryClient();

  function handleOpenChange(next: boolean) {
    setOpen(next);
    if (!next) {
      setResults(null);
      setTargetIds(new Set());
    }
  }

  const aliveParticipants = participants.filter(
    (p) => (p.state.currentState.hp ?? 1) > 0,
  );

  function toggleTarget(id: string) {
    setTargetIds((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function attack() {
    if (!attackerId) return toast.error("Choisissez un attaquant");
    if (targetIds.size === 0) return toast.error("Choisissez au moins une cible");
    setBusy(true);
    try {
      const res = await fetch(`/api/sessions/${sessionId}/actions`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          kind: "attack",
          attackerId,
          targetIds: Array.from(targetIds),
          attackBonus: Number(attackBonus),
          damageNotation,
          damageType,
          meleeWithin5ft,
        }),
      });
      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error?.message ?? "Échec de l’attaque");
      }
      const { records } = (await res.json()) as { records: ResolutionRecord[] };
      setResults(records);
      queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
      queryClient.invalidateQueries({ queryKey: ["timeline", sessionId] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Échec de l’attaque");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>{trigger}</DialogTrigger>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>Attaquer</DialogTitle>
        </DialogHeader>

        {results ? (
          <div className="space-y-2" data-testid="attack-results">
            <h3 className="text-sm font-semibold">Résolution</h3>
            <ScrollArea className="max-h-72 rounded border p-3">
              <ul className="space-y-1 text-sm">
                {results.map((r, i) => (
                  <li key={i} className="border-l-2 pl-2 py-0.5">
                    <Badge
                      variant={
                        r.outcome === "success"
                          ? "default"
                          : r.outcome === "fail"
                          ? "destructive"
                          : "outline"
                      }
                      className="mr-2 capitalize"
                    >
                      {OUTCOME_LABELS[r.outcome]}
                    </Badge>
                    {r.description}
                  </li>
                ))}
              </ul>
            </ScrollArea>
            <DialogFooter>
              <Button variant="outline" onClick={() => setResults(null)}>
                Nouvelle attaque
              </Button>
              <Button onClick={() => handleOpenChange(false)}>Fermer</Button>
            </DialogFooter>
          </div>
        ) : (
          <div className="space-y-3">
            <div className="space-y-1">
              <Label className="text-xs uppercase">Attaquant</Label>
              <Select value={attackerId} onValueChange={setAttackerId}>
                <SelectTrigger>
                  <SelectValue placeholder="Choisir un attaquant…" />
                </SelectTrigger>
                <SelectContent>
                  {aliveParticipants.map((p) => (
                    <SelectItem key={p.state.entityId} value={p.state.entityId}>
                      {p.entity?.name ?? p.state.entityId}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div>
              <Label className="text-xs uppercase">Cible(s)</Label>
              <div className="rounded border p-2 max-h-40 overflow-auto space-y-1">
                {aliveParticipants
                  .filter((p) => p.state.entityId !== attackerId)
                  .map((p) => (
                    <label
                      key={p.state.entityId}
                      className="flex items-center gap-2 px-1 py-1 rounded hover:bg-muted cursor-pointer"
                    >
                      <Checkbox
                        checked={targetIds.has(p.state.entityId)}
                        onCheckedChange={() => toggleTarget(p.state.entityId)}
                      />
                      <span className="flex-1 text-sm">{p.entity?.name}</span>
                      {p.state.currentState.ac !== undefined ? (
                        <Badge variant="outline" className="text-xs">
                          AC {p.state.currentState.ac}
                        </Badge>
                      ) : null}
                      {p.state.currentState.conditions.length > 0 ? (
                        <Badge variant="destructive" className="text-xs">
                          {p.state.currentState.conditions[0].conditionId}
                          {p.state.currentState.conditions.length > 1 ? "+" : ""}
                        </Badge>
                      ) : null}
                    </label>
                  ))}
              </div>
            </div>

            <div className="grid gap-3 grid-cols-2">
              <div className="space-y-1">
                <Label className="text-xs uppercase">Bonus d’attaque</Label>
                <Input
                  type="number"
                  value={attackBonus}
                  onChange={(e) => setAttackBonus(e.target.value)}
                />
              </div>
              <div className="space-y-1">
                <Label className="text-xs uppercase">Type de dégâts</Label>
                <Select value={damageType} onValueChange={setDamageType}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ruleset.damageTypes.map((d) => (
                      <SelectItem key={d.id} value={d.id}>
                        {d.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1 col-span-2">
                <Label className="text-xs uppercase">Dés de dégâts</Label>
                <Input
                  value={damageNotation}
                  onChange={(e) => setDamageNotation(e.target.value)}
                  placeholder="1d8+3"
                />
              </div>
              <label className="flex items-center gap-2 col-span-2">
                <Checkbox
                  checked={meleeWithin5ft}
                  onCheckedChange={(v) => setMeleeWithin5ft(!!v)}
                />
                <span className="text-sm">Au contact à 1,5 m (critique automatique sur une cible paralysée/inconsciente)</span>
              </label>
            </div>

            <DialogFooter>
              <Button onClick={attack} disabled={busy} data-testid="attack-submit">
                {busy ? "Résolution…" : "Résoudre l’attaque"}
              </Button>
            </DialogFooter>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
