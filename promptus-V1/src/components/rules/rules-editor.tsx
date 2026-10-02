"use client";

import { useState } from "react";
import * as yaml from "js-yaml";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Textarea } from "@/components/ui/textarea";
import { PHASE_LABELS } from "@/lib/engine/catalog";
import { abilityLabel, skillLabel, type Ruleset } from "@/lib/engine/ruleset";
import { rulesetQueryKey } from "@/components/providers/ruleset-provider";

interface RulesResponse {
  ruleset: Ruleset;
  isPreset: boolean;
}

interface ApiIssue {
  path: (string | number)[];
  message: string;
}

const MODE_LABELS = {
  roll_over: "au-dessus (total ≥ difficulté)",
  roll_under: "en dessous (dés ≤ valeur − difficulté)",
} as const;

const MODIFIER_LABELS = {
  dnd: "⌊(valeur − 10) / 2⌋",
  direct: "la valeur elle-même",
  none: "aucun",
} as const;

const DIAGONAL_LABELS = {
  chebyshev: "1 case",
  alternate: "1 puis 2 cases",
  euclidean: "√2",
} as const;

export function RulesEditor({ campaignId }: { campaignId: string }) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<string | null>(null);
  const [issues, setIssues] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);

  const { data, isLoading, error } = useQuery({
    queryKey: rulesetQueryKey(campaignId),
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/rules`);
      if (!res.ok) throw new Error("Impossible de charger les règles");
      return (await res.json()) as RulesResponse;
    },
  });

  if (error) return <p className="text-destructive">{(error as Error).message}</p>;
  if (isLoading || !data) return <p className="text-muted-foreground">Chargement des règles…</p>;

  const { ruleset, isPreset } = data;
  const text = draft ?? yaml.dump(ruleset, { lineWidth: 120, noRefs: true });

  async function put(body: object, okMessage: string) {
    setBusy(true);
    setIssues([]);
    try {
      const res = await fetch(`/api/campaigns/${campaignId}/rules`, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      });
      const json = await res.json();
      if (!res.ok) {
        const details = (json.error?.details ?? []) as ApiIssue[];
        // Les erreurs de validation s'affichent sous l'éditeur, pas en toast
        // (qui masquerait le bouton d'enregistrement).
        if (details.length > 0) {
          setIssues(details.map((d) => `${d.path.join(".") || "(racine)"} : ${d.message}`));
          return;
        }
        throw new Error(json.error?.message ?? "Échec de l’enregistrement");
      }
      queryClient.setQueryData(rulesetQueryKey(campaignId), json as RulesResponse);
      setDraft(null);
      toast.success(okMessage);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Erreur inconnue");
    } finally {
      setBusy(false);
    }
  }

  function save() {
    let parsed: unknown;
    try {
      parsed = yaml.load(text);
    } catch (e) {
      setIssues([`YAML invalide : ${e instanceof Error ? e.message : String(e)}`]);
      return;
    }
    void put({ ruleset: parsed }, "Règles enregistrées");
  }

  const phases = Object.keys(PHASE_LABELS) as (keyof typeof PHASE_LABELS)[];

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-2">
        <h2 className="text-xl font-semibold">{ruleset.name}</h2>
        <Badge variant={isPreset ? "secondary" : "default"}>
          {isPreset ? "Préréglage" : "Personnalisé"}
        </Badge>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Tests</CardTitle>
          </CardHeader>
          <CardContent className="text-sm space-y-1">
            <p>Dés : <strong>{ruleset.check.dice}</strong>, jet {MODE_LABELS[ruleset.check.mode]}</p>
            <p>Modificateur de caractéristique : {MODIFIER_LABELS[ruleset.check.abilityModifier]}</p>
            <p>
              Critique sur {ruleset.check.criticalOn ?? "—"}, échec critique sur {ruleset.check.fumbleOn ?? "—"}
              {ruleset.check.advantage ? " · avantage / désavantage" : ""}
            </p>
            <p>
              Initiative : {ruleset.initiative.dice}
              {ruleset.initiative.ability ? ` + ${abilityLabel(ruleset, ruleset.initiative.ability)}` : ""}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Déplacement</CardTitle>
          </CardHeader>
          <CardContent className="text-sm space-y-1">
            <p>
              Campagne : 1 case = {ruleset.movement.campaign.cellKm} km ·{" "}
              {ruleset.movement.campaign.paces.map((p) => `${p.label} ${p.cellsPerDay}/jour`).join(", ")}
            </p>
            <p>
              Région : 1 case = {ruleset.movement.region.cellMeters} m · {ruleset.movement.region.cellsPerHour} cases/heure
            </p>
            <p>
              Combat : 1 case = {ruleset.movement.local.cellMeters} m · vitesse {ruleset.movement.local.defaultSpeedCells} cases
              · diagonale {DIAGONAL_LABELS[ruleset.movement.local.diagonal]} · round de {ruleset.combat.roundSeconds} s
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Caractéristiques et compétences</CardTitle>
          </CardHeader>
          <CardContent className="text-sm space-y-2">
            <div className="flex flex-wrap gap-1">
              {ruleset.abilities.map((a) => (
                <Badge key={a.id} variant="outline" title={a.id}>
                  {a.abbr} · {a.label}
                </Badge>
              ))}
            </div>
            <p className="text-muted-foreground">
              {ruleset.skills.map((s) => `${s.label} (${abilityLabel(ruleset, s.ability)})`).join(", ") || "Aucune compétence"}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">États, dégâts, ressources</CardTitle>
          </CardHeader>
          <CardContent className="text-sm space-y-1 text-muted-foreground">
            <p><span className="text-foreground">{ruleset.conditions.length} états :</span> {ruleset.conditions.map((c) => c.name).join(", ")}</p>
            <p><span className="text-foreground">Dégâts :</span> {ruleset.damageTypes.map((d) => d.label).join(", ")}</p>
            <p><span className="text-foreground">Ressources :</span> {ruleset.resources.map((r) => r.label).join(", ")}</p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Actions proposées aux joueurs</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-3 md:grid-cols-5 text-sm">
          {phases.map((phase) => (
            <div key={phase}>
              <p className="text-xs uppercase tracking-wide text-muted-foreground mb-1">{PHASE_LABELS[phase]}</p>
              <ul className="space-y-0.5">
                {ruleset.actions
                  .filter((a) => a.phases.includes(phase))
                  .map((a) => (
                    <li key={a.id} title={a.description}>
                      {a.label}
                      {a.skill ? <span className="text-muted-foreground"> ({skillLabel(ruleset, a.skill)})</span> : null}
                    </li>
                  ))}
              </ul>
            </div>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Modifier les règles (YAML)</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-sm text-muted-foreground">
            Les identifiants (<code>id</code>) sont ceux utilisés par les fiches et les effets : changez les
            libellés librement, mais renommer un id casse les références existantes.
          </p>
          <Textarea
            value={text}
            onChange={(e) => setDraft(e.target.value)}
            className="font-mono text-xs h-[60vh] field-sizing-fixed overflow-auto"
            spellCheck={false}
            aria-label="Règles au format YAML"
          />
          {issues.length > 0 ? (
            <ul className="text-sm text-destructive list-disc pl-5" data-testid="rules-issues">
              {issues.map((i) => (
                <li key={i}>{i}</li>
              ))}
            </ul>
          ) : null}
          <div className="flex flex-wrap justify-between gap-2">
            <Button
              variant="outline"
              disabled={busy || isPreset}
              onClick={() => put({ reset: true }, "Préréglage D&D 5e rétabli")}
            >
              Revenir au préréglage D&D 5e
            </Button>
            <div className="flex gap-2">
              <Button variant="ghost" disabled={busy || draft === null} onClick={() => { setDraft(null); setIssues([]); }}>
                Annuler les modifications
              </Button>
              <Button disabled={busy || draft === null} onClick={save}>
                {busy ? "Enregistrement…" : "Enregistrer"}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
