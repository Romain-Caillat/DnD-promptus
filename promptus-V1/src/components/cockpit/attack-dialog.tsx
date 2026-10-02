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
import { creatureAttacks, type AttackGeometry, type CreatureAttack } from "@/lib/engine/combat";
import { damageTypeLabel } from "@/lib/engine/ruleset";
import { useRuleset } from "@/components/providers/ruleset-provider";
import type { ParticipantView } from "@/lib/stores/session-store";
import type { ResolutionRecord } from "@/lib/engine/types";

const MANUAL = "__manual";

interface Props {
  sessionId: string;
  participants: ParticipantView[];
  trigger?: React.ReactNode;
  /** Ouverture pilotée (depuis la carte). */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  preset?: { attackerId: string; targetIds: string[] };
  /** Portée et ligne de vue sur la carte affichée, si les deux pions y sont. */
  geometry?: (attackerId: string, targetId: string, attack: Pick<CreatureAttack, "rangeMeters" | "longRangeMeters">) => AttackGeometry | null;
}

export function AttackDialog({ sessionId, participants, trigger, open: openProp, onOpenChange, preset, geometry }: Props) {
  const ruleset = useRuleset();
  const [openState, setOpenState] = useState(false);
  const open = openProp ?? openState;
  const [attackerId, setAttackerId] = useState<string>(preset?.attackerId ?? "");
  const [targetIds, setTargetIds] = useState<Set<string>>(new Set(preset?.targetIds ?? []));
  const [attackChoice, setAttackChoice] = useState<string>("");
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
    setOpenState(next);
    onOpenChange?.(next);
    if (!next) {
      setResults(null);
      if (!preset) setTargetIds(new Set());
    }
  }

  const aliveParticipants = participants.filter((p) => (p.state.currentState.hp ?? 1) > 0);
  const attacker = participants.find((p) => p.state.entityId === attackerId);
  const attacks = attacker?.entity ? creatureAttacks(attacker.entity.attributes, ruleset) : [];
  // Par défaut : la première attaque de la fiche ; « saisie libre » sinon.
  const attackId = attackChoice || attacks[0]?.id || MANUAL;
  const weapon = attacks.find((a) => a.id === attackId);
  const geoFor = (targetId: string) =>
    geometry && attackerId ? geometry(attackerId, targetId, weapon ?? { rangeMeters: Infinity }) : null;

  function toggleTarget(id: string) {
    setTargetIds((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function attack(force = false) {
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
          ...(weapon
            ? { attackId: weapon.id }
            : { attackBonus: Number(attackBonus), damageNotation, damageType, meleeWithin5ft }),
          force,
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

  const blocked = weapon ? [...targetIds].some((t) => geoFor(t)?.reason) : false;

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      {trigger ? <DialogTrigger asChild>{trigger}</DialogTrigger> : null}
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
                      variant={r.outcome === "success" ? "default" : r.outcome === "fail" ? "destructive" : "outline"}
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
              <Select
                value={attackerId}
                onValueChange={(v) => {
                  setAttackerId(v);
                  setAttackChoice("");
                }}
              >
                <SelectTrigger aria-label="Attaquant">
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

            {attacker ? (
              <div className="space-y-1">
                <Label className="text-xs uppercase">Attaque</Label>
                <Select value={attackId} onValueChange={setAttackChoice}>
                  <SelectTrigger aria-label="Attaque">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {attacks.map((a) => (
                      <SelectItem key={a.id} value={a.id}>
                        {a.name} ({a.bonus >= 0 ? "+" : ""}
                        {a.bonus}, {a.damage} {damageTypeLabel(ruleset, a.damageType)},{" "}
                        {a.longRangeMeters ? `${a.rangeMeters}/${a.longRangeMeters} m` : `${a.rangeMeters} m`})
                      </SelectItem>
                    ))}
                    <SelectItem value={MANUAL}>Saisie libre…</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            ) : null}

            <div>
              <Label className="text-xs uppercase">Cible(s)</Label>
              <div className="rounded border p-2 max-h-40 overflow-auto space-y-1">
                {aliveParticipants
                  .filter((p) => p.state.entityId !== attackerId)
                  .map((p) => {
                    const geo = geoFor(p.state.entityId);
                    return (
                      <label
                        key={p.state.entityId}
                        className="flex items-center gap-2 px-1 py-1 rounded hover:bg-muted cursor-pointer"
                      >
                        <Checkbox
                          checked={targetIds.has(p.state.entityId)}
                          onCheckedChange={() => toggleTarget(p.state.entityId)}
                        />
                        <span className="flex-1 text-sm">{p.entity?.name}</span>
                        {geo ? (
                          <Badge
                            variant={geo.reason ? "destructive" : geo.longRange ? "secondary" : "outline"}
                            className="text-xs"
                            title={geo.reason ?? undefined}
                          >
                            {geo.distanceMeters.toLocaleString("fr-FR")} m
                            {geo.reason ? (geo.inRange ? " · pas de vue" : " · hors portée") : geo.longRange ? " · désavantage" : ""}
                          </Badge>
                        ) : null}
                        {p.state.currentState.ac !== undefined ? (
                          <Badge variant="outline" className="text-xs">
                            CA {p.state.currentState.ac}
                          </Badge>
                        ) : null}
                      </label>
                    );
                  })}
              </div>
            </div>

            {!weapon ? (
              <div className="grid gap-3 grid-cols-2">
                <div className="space-y-1">
                  <Label className="text-xs uppercase">Bonus d’attaque</Label>
                  <Input type="number" value={attackBonus} onChange={(e) => setAttackBonus(e.target.value)} />
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
                  <Input value={damageNotation} onChange={(e) => setDamageNotation(e.target.value)} placeholder="1d8+3" />
                </div>
                <label className="flex items-center gap-2 col-span-2">
                  <Checkbox checked={meleeWithin5ft} onCheckedChange={(v) => setMeleeWithin5ft(!!v)} />
                  <span className="text-sm">
                    Au contact à 1,5 m (critique automatique sur une cible paralysée/inconsciente)
                  </span>
                </label>
              </div>
            ) : null}

            <DialogFooter>
              {blocked ? (
                <Button variant="outline" onClick={() => attack(true)} disabled={busy} title="Ignorer la portée et la ligne de vue">
                  Forcer
                </Button>
              ) : null}
              <Button onClick={() => attack(false)} disabled={busy || blocked} data-testid="attack-submit">
                {busy ? "Résolution…" : "Résoudre l’attaque"}
              </Button>
            </DialogFooter>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
