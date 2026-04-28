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
    if (!name.trim()) return toast.error("Campaign name is required");
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
      if (!res.ok) throw new Error((await res.json()).error?.message ?? "Save failed");
      toast.success("Campaign settings saved");
      router.refresh();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Save failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Identity</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="space-y-1">
            <Label htmlFor="cname" className="text-xs uppercase tracking-wide text-muted-foreground">
              Name
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
          <CardTitle className="text-lg">Visual style guide</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <Field
            label="Art style"
            help="e.g. dark fantasy oil painting, ink wash, low-poly 3D"
            value={style.artStyle ?? ""}
            onChange={(v) => set("artStyle", v)}
          />
          <Field
            label="Mood"
            help="e.g. ominous, candle-lit, epic, somber"
            value={style.mood ?? ""}
            onChange={(v) => set("mood", v)}
          />
          <Field
            label="Palette"
            help="e.g. muted earth tones, neon cyberpunk, monochrome"
            value={style.palette ?? ""}
            onChange={(v) => set("palette", v)}
          />
          <Field
            label="Prompt prefix"
            help="Prepended to every entity prompt — locks in the global aesthetic."
            value={style.promptPrefix ?? ""}
            onChange={(v) => set("promptPrefix", v)}
            multiline
          />
          <Field
            label="Prompt suffix"
            help="Appended last — useful for technical hints (lighting, framing)."
            value={style.promptSuffix ?? ""}
            onChange={(v) => set("promptSuffix", v)}
            multiline
          />
          <Field
            label="Negative prompt"
            help="Comma-separated tokens you NEVER want in the image."
            value={style.negativePrompt ?? ""}
            onChange={(v) => set("negativePrompt", v)}
            multiline
          />
        </CardContent>
      </Card>

      <Button onClick={save} disabled={busy} size="lg">
        {busy ? "Saving…" : "Save settings"}
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
