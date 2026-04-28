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
import { ENTITY_TYPES, VISIBILITIES } from "@/lib/engine/catalog";
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
      toast.error("Name is required");
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
        if (!res.ok) throw new Error((await res.json()).error?.message ?? "Save failed");
        const data = await res.json();
        toast.success(`Entity "${data.entity.name}" created`);
        router.push(`/campaigns/${campaignId}/entities`);
        router.refresh();
      } else {
        if (!state.id) throw new Error("Missing entity id for edit");
        const res = await fetch(`/api/entities/${state.id}`, {
          method: "PATCH",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(payload),
        });
        if (!res.ok) throw new Error((await res.json()).error?.message ?? "Save failed");
        toast.success(`Entity "${state.name}" updated`);
        router.push(`/campaigns/${campaignId}/entities`);
        router.refresh();
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Unknown error");
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (mode !== "edit" || !state.id) return;
    setBusy(true);
    try {
      const res = await fetch(`/api/entities/${state.id}`, { method: "DELETE" });
      if (!res.ok) throw new Error((await res.json()).error?.message ?? "Delete failed");
      toast.success(`Entity "${state.name}" deleted`);
      router.push(`/campaigns/${campaignId}/entities`);
      router.refresh();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Unknown error");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      {/* Base section */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Identity</CardTitle>
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
                Visibility
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
                Name
              </Label>
              <Input
                id="ent-name"
                value={state.name}
                onChange={(e) => update("name", e.target.value)}
                placeholder="Fireball, Bortrand the Robust, The Forgotten Crypt…"
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
                placeholder="A short narrative description; players see this if visibility allows."
                rows={3}
              />
            </div>
            <div className="md:col-span-2 space-y-1">
              <Label htmlFor="ent-tags" className="text-xs uppercase tracking-wide text-muted-foreground">
                Tags (comma-separated)
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
                placeholder="evocation, fire, AoE, level 3"
              />
            </div>
            <div className="md:col-span-2 space-y-1">
              <div className="flex items-center justify-between">
                <Label htmlFor="ent-img" className="text-xs uppercase tracking-wide text-muted-foreground">
                  Image URL (optional)
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
                placeholder="/generated-images/… or https://…"
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
          <CardTitle className="text-lg capitalize">{state.type} attributes</CardTitle>
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
          <CardTitle className="text-lg">Effects</CardTitle>
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
          Cancel
        </Button>
        <div className="flex gap-2">
          {mode === "edit" && (
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button type="button" variant="destructive" disabled={busy}>
                  Delete
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Delete this entity?</AlertDialogTitle>
                  <AlertDialogDescription>
                    This action cannot be undone. The entity “{state.name}” will be
                    permanently removed from this campaign.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Cancel</AlertDialogCancel>
                  <AlertDialogAction onClick={remove}>Delete</AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          )}
          <Button type="button" onClick={save} disabled={busy}>
            {busy ? "Saving…" : mode === "create" ? "Create entity" : "Save changes"}
          </Button>
        </div>
      </div>
    </div>
  );
}
