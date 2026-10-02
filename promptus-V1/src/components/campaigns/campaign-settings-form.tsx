"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { StyleGuide } from "@/lib/engine/types";

interface Props {
  campaignId: string;
  initial: {
    name: string;
    description: string;
    styleGuide: StyleGuide;
  };
}

export function CampaignSettingsForm({ campaignId, initial }: Props) {
  const [name, setName] = useState(initial.name);
  const [description, setDescription] = useState(initial.description);
  const [style, setStyle] = useState<StyleGuide>(initial.styleGuide);
  const [busy, setBusy] = useState(false);
  const router = useRouter();

  function set<K extends keyof StyleGuide>(key: K, value: StyleGuide[K]) {
    setStyle((s) => ({ ...s, [key]: value }));
  }

  async function save() {
    if (!name.trim()) return toast.error("Le nom de la campagne est obligatoire");
    setBusy(true);
    try {
      const res = await fetch(`/api/campaigns/${campaignId}`, {
        method: "PATCH",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          name: name.trim(),
          description: description.trim() || undefined,
          styleGuide: style,
        }),
      });
      if (!res.ok) throw new Error((await res.json()).error?.message ?? "Échec de l’enregistrement");
      toast.success("Paramètres de la campagne enregistrés");
      router.refresh();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Échec de l’enregistrement");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Identité</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-1">
            <Label htmlFor="cname" className="text-xs uppercase tracking-wide text-muted-foreground">
              Nom
            </Label>
            <Input id="cname" value={name} onChange={(e) => setName(e.target.value)} />
          </div>
          <div className="space-y-1">
            <Label htmlFor="cdesc" className="text-xs uppercase tracking-wide text-muted-foreground">
              Description
            </Label>
            <Textarea
              id="cdesc"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={2}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Guide de style visuel</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <Field
            label="Style artistique"
            help="ex. peinture à l’huile dark fantasy, lavis d’encre, 3D low-poly"
            value={style.artStyle ?? ""}
            onChange={(v) => set("artStyle", v)}
          />
          <Field
            label="Ambiance"
            help="ex. inquiétante, à la chandelle, épique, sombre"
            value={style.mood ?? ""}
            onChange={(v) => set("mood", v)}
          />
          <Field
            label="Palette"
            help="ex. tons terreux sourds, néons cyberpunk, monochrome"
            value={style.palette ?? ""}
            onChange={(v) => set("palette", v)}
          />
          <Field
            label="Préfixe du prompt"
            help="Ajouté au début de chaque prompt : fixe l’esthétique globale."
            value={style.promptPrefix ?? ""}
            onChange={(v) => set("promptPrefix", v)}
            multiline
          />
          <Field
            label="Suffixe du prompt"
            help="Ajouté à la fin : utile pour les indications techniques (lumière, cadrage)."
            value={style.promptSuffix ?? ""}
            onChange={(v) => set("promptSuffix", v)}
            multiline
          />
          <Field
            label="Prompt négatif"
            help="Éléments à ne JAMAIS voir dans l’image, séparés par des virgules."
            value={style.negativePrompt ?? ""}
            onChange={(v) => set("negativePrompt", v)}
            multiline
          />
        </CardContent>
      </Card>

      <Button onClick={save} disabled={busy} size="lg">
        {busy ? "Enregistrement…" : "Enregistrer"}
      </Button>
    </div>
  );
}

function Field({
  label,
  help,
  value,
  onChange,
  multiline,
}: {
  label: string;
  help?: string;
  value: string;
  onChange: (v: string) => void;
  multiline?: boolean;
}) {
  const id = `f-${label.toLowerCase().replace(/\s+/g, "-")}`;
  return (
    <div className="space-y-1">
      <Label htmlFor={id} className="text-xs uppercase tracking-wide text-muted-foreground">
        {label}
      </Label>
      {multiline ? (
        <Textarea id={id} value={value} onChange={(e) => onChange(e.target.value)} rows={2} />
      ) : (
        <Input id={id} value={value} onChange={(e) => onChange(e.target.value)} />
      )}
      {help ? <p className="text-xs text-muted-foreground italic">{help}</p> : null}
    </div>
  );
}
