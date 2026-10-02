"use client";

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { AiSettings } from "@/lib/db/schema";

interface ModelOption {
  id: string;
  name: string;
  promptPerM: number;
  completionPerM: number;
}

export function AiSettingsCard({
  campaignId,
  initial,
  defaultModel,
}: {
  campaignId: string;
  initial: AiSettings;
  defaultModel: string;
}) {
  const [model, setModel] = useState(initial.model ?? "");
  const [budget, setBudget] = useState(initial.budgetUsd !== undefined ? String(initial.budgetUsd) : "");
  const [busy, setBusy] = useState(false);

  const { data } = useQuery({
    queryKey: ["ai-models"],
    queryFn: async () => (await (await fetch("/api/ai/models")).json()) as { models: ModelOption[]; error?: string },
    staleTime: 3_600_000,
  });
  const selected = data?.models.find((m) => m.id === (model || defaultModel));

  async function save() {
    setBusy(true);
    try {
      const res = await fetch(`/api/campaigns/${campaignId}/ai-settings`, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          model: model.trim() || undefined,
          budgetUsd: budget.trim() === "" ? undefined : Number(budget),
        }),
      });
      if (!res.ok) throw new Error((await res.json()).error?.message ?? "Échec de l’enregistrement");
      toast.success("Réglages IA enregistrés");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Erreur inconnue");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">Intelligence artificielle</CardTitle>
        <CardDescription>Modèle OpenRouter utilisé pour la génération, et budget total de la campagne.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="space-y-1">
          <Label htmlFor="ai-model" className="text-xs uppercase tracking-wide text-muted-foreground">
            Modèle
          </Label>
          <Input
            id="ai-model"
            list="ai-model-options"
            value={model}
            onChange={(e) => setModel(e.target.value)}
            placeholder={`Par défaut : ${defaultModel}`}
          />
          <datalist id="ai-model-options">
            {(data?.models ?? []).map((m) => (
              <option key={m.id} value={m.id}>
                {m.name}
              </option>
            ))}
          </datalist>
          <p className="text-xs text-muted-foreground">
            {selected
              ? `${selected.name} : ${selected.promptPerM.toFixed(2)} $ / M jetons en entrée, ${selected.completionPerM.toFixed(2)} $ / M en sortie.`
              : data?.error ?? "Choisissez un identifiant de modèle OpenRouter (ex. fournisseur/modèle)."}
          </p>
        </div>
        <div className="space-y-1">
          <Label htmlFor="ai-budget" className="text-xs uppercase tracking-wide text-muted-foreground">
            Budget total (dollars)
          </Label>
          <Input
            id="ai-budget"
            type="number"
            min={0}
            step="0.5"
            value={budget}
            onChange={(e) => setBudget(e.target.value)}
            placeholder="Sans limite"
          />
          <p className="text-xs text-muted-foreground">
            Une génération s’arrête dès que le budget restant est dépassé ; aucune ne démarre une fois le budget épuisé.
          </p>
        </div>
        <div className="flex justify-end">
          <Button onClick={save} disabled={busy}>
            {busy ? "Enregistrement…" : "Enregistrer"}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
