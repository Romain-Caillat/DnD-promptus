"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { ENTITY_TYPES, ENTITY_TYPE_LABELS, VISIBILITIES } from "@/lib/engine/catalog";
import type { Effect, EntityType, Visibility } from "@/lib/engine/types";
import { EffectBuilder } from "./effect-builder";
import { TypeAttributes } from "./type-attributes";
import { GenerateImageButton } from "./generate-image-button";

export interface EntityFormState {
  id?: string;
  type: EntityType;
  name: string;
  description: string;
  imageUrl: string;
  tags: string[];
  attributes: Record<string, unknown>;
  effects: Effect[];
  visibility: Visibility;
}

const EMPTY: EntityFormState = {
  type: "spell",
  name: "",
  description: "",
  imageUrl: "",
  tags: [],
  attributes: {},
  effects: [],
  visibility: "public",
};

function buildDefaultPrompt(state: EntityFormState): string {
  // Lightweight client-side prompt — the server adds the campaign style guide.
  const subject = state.description?.trim()
    ? `${state.type}: ${state.name}, ${state.description.trim()}`
    : `${state.type}: ${state.name || "(unnamed)"}`;
  const tagPart = state.tags.length ? `, ${state.tags.slice(0, 6).join(", ")}` : "";
  return `${subject}${tagPart}`;
}

interface Props {
  campaignId: string;
  initial?: EntityFormState;
  mode: "create" | "edit";
}

export function EntityEditor({ campaignId, initial, mode }: Props) {
  const router = useRouter();
  const [state, setState] = useState<EntityFormState>(initial ?? EMPTY);
  const [busy, setBusy] = useState(false);

  function update<K extends keyof EntityFormState>(key: K, value: EntityFormState[K]) {
    setState((s) => ({ ...s, [key]: value }));
  }

  async function save() {
    if (!state.name.trim()) {
      toast.error("Le nom est obligatoire");
      return;
    }
    setBusy(true);
    try {
      const payload = {
        type: state.type,
        name: state.name.trim(),
        description: state.description.trim() || undefined,
        imageUrl: state.imageUrl.trim() || undefined,
        tags: state.tags,
        attributes: state.attributes,
        effects: state.effects,
        visibility: state.visibility,
      };

      if (mode === "create") {
        const res = await fetch(`/api/campaigns/${campaignId}/entities`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(payload),
        });
        if (!res.ok) throw new Error((await res.json()).error?.message ?? "Échec de l’enregistrement");
        const data = await res.json();
        toast.success(`Fiche « ${data.entity.name} » créée`);
        router.push(`/campaigns/${campaignId}/entities`);
        router.refresh();
      } else {
        if (!state.id) throw new Error("ID de fiche manquant");
        const res = await fetch(`/api/entities/${state.id}`, {
          method: "PATCH",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(payload),
        });
        if (!res.ok) throw new Error((await res.json()).error?.message ?? "Échec de l’enregistrement");
        toast.success(`Fiche « ${state.name} » mise à jour`);
        router.push(`/campaigns/${campaignId}/entities`);
        router.refresh();
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Erreur inconnue");
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (mode !== "edit" || !state.id) return;
    setBusy(true);
    try {
      const res = await fetch(`/api/entities/${state.id}`, { method: "DELETE" });
      if (!res.ok) throw new Error((await res.json()).error?.message ?? "Échec de la suppression");
      toast.success(`Fiche « ${state.name} » supprimée`);
      router.push(`/campaigns/${campaignId}/entities`);
      router.refresh();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Erreur inconnue");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      {/* Base section */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Identité</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-3 md:grid-cols-2">
            <div className="space-y-1">
              <Label htmlFor="ent-type" className="text-xs uppercase tracking-wide text-muted-foreground">
                Type
              </Label>
              <Select
                value={state.type}
                onValueChange={(v) => update("type", v as EntityType)}
                disabled={mode === "edit"}
              >
                <SelectTrigger id="ent-type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {ENTITY_TYPES.map((t) => (
                    <SelectItem key={t.id} value={t.id}>
                      {t.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1">
              <Label htmlFor="ent-vis" className="text-xs uppercase tracking-wide text-muted-foreground">
                Visibilité
              </Label>
              <Select value={state.visibility} onValueChange={(v) => update("visibility", v as Visibility)}>
                <SelectTrigger id="ent-vis">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {VISIBILITIES.map((v) => (
                    <SelectItem key={v.id} value={v.id}>
                      {v.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="md:col-span-2 space-y-1">
              <Label htmlFor="ent-name" className="text-xs uppercase tracking-wide text-muted-foreground">
                Nom
              </Label>
              <Input
                id="ent-name"
                value={state.name}
                onChange={(e) => update("name", e.target.value)}
                placeholder="Boule de feu, Bortrand le Robuste, La Crypte oubliée…"
                autoFocus={mode === "create"}
                required
              />
            </div>
            <div className="md:col-span-2 space-y-1">
              <Label htmlFor="ent-desc" className="text-xs uppercase tracking-wide text-muted-foreground">
                Description
              </Label>
              <Textarea
                id="ent-desc"
                value={state.description}
                onChange={(e) => update("description", e.target.value)}
                placeholder="Courte description narrative ; visible des joueurs si la visibilité le permet."
                rows={3}
              />
            </div>
            <div className="md:col-span-2 space-y-1">
              <Label htmlFor="ent-tags" className="text-xs uppercase tracking-wide text-muted-foreground">
                Tags (séparés par des virgules)
              </Label>
              <Input
                id="ent-tags"
                value={state.tags.join(", ")}
                onChange={(e) =>
                  update(
                    "tags",
                    e.target.value
                      .split(",")
                      .map((s) => s.trim())
                      .filter(Boolean),
                  )
                }
                placeholder="évocation, feu, zone, niveau 3"
              />
            </div>
            <div className="md:col-span-2 space-y-1">
              <div className="flex items-center justify-between">
                <Label htmlFor="ent-img" className="text-xs uppercase tracking-wide text-muted-foreground">
                  URL de l’image (facultative)
                </Label>
                <GenerateImageButton
                  entityId={state.id ?? null}
                  defaultPrompt={buildDefaultPrompt(state)}
                  onGenerated={(url) => update("imageUrl", url)}
                />
              </div>
              <Input
                id="ent-img"
                value={state.imageUrl}
                onChange={(e) => update("imageUrl", e.target.value)}
                placeholder="/api/media/… ou https://…"
              />
              {state.imageUrl ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img
                  src={state.imageUrl}
                  alt={state.name}
                  className="rounded border max-h-48 object-contain bg-black/30 mt-2"
                />
              ) : null}
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Type attributes */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Attributs : {ENTITY_TYPE_LABELS[state.type]}</CardTitle>
        </CardHeader>
        <CardContent>
          <TypeAttributes
            type={state.type}
            attributes={state.attributes}
            onChange={(next) => update("attributes", next)}
          />
        </CardContent>
      </Card>

      {/* Effects */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Effets</CardTitle>
        </CardHeader>
        <CardContent>
          <EffectBuilder value={state.effects} onChange={(next) => update("effects", next)} />
        </CardContent>
      </Card>

      <Separator />

      {/* Actions */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <Button
          type="button"
          variant="outline"
          onClick={() => router.push(`/campaigns/${campaignId}/entities`)}
        >
          Annuler
        </Button>
        <div className="flex gap-2">
          {mode === "edit" && (
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button type="button" variant="destructive" disabled={busy}>
                  Supprimer
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Supprimer cette fiche ?</AlertDialogTitle>
                  <AlertDialogDescription>
                    Cette action est irréversible. La fiche « {state.name} » sera
                    définitivement retirée de cette campagne.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Annuler</AlertDialogCancel>
                  <AlertDialogAction onClick={remove}>Supprimer</AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          )}
          <Button type="button" onClick={save} disabled={busy}>
            {busy ? "Enregistrement…" : mode === "create" ? "Créer la fiche" : "Enregistrer"}
          </Button>
        </div>
      </div>
    </div>
  );
}
