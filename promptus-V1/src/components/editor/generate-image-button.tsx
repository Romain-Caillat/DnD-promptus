"use client";

import { useState } from "react";
import { Sparkles } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

interface Props {
  /** If null, the entity hasn't been saved yet — disable the button. */
  entityId: string | null;
  defaultPrompt: string;
  onGenerated: (url: string) => void;
}

export function GenerateImageButton({ entityId, defaultPrompt, onGenerated }: Props) {
  const [open, setOpen] = useState(false);
  const [prompt, setPrompt] = useState(defaultPrompt);
  const [busy, setBusy] = useState(false);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);

  async function generate() {
    if (!entityId) return toast.error("Enregistrez d’abord la fiche pour générer une image");
    if (!prompt.trim()) return toast.error("Le prompt ne peut pas être vide");
    setBusy(true);
    try {
      const res = await fetch(`/api/ai/image`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ entityId, prompt: prompt.trim() }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error?.message ?? "Échec de la génération");
      }
      setPreviewUrl(data.result.url);
      onGenerated(data.result.url);
      toast.success(`Image générée (${data.result.latencyMs} ms)`);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Échec de la génération");
    } finally {
      setBusy(false);
    }
  }

  function close() {
    setOpen(false);
    setPreviewUrl(null);
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={!entityId}
          title={!entityId ? "Enregistrez d’abord la fiche" : undefined}
        >
          <Sparkles className="size-4" /> Générer
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>Générer une image</DialogTitle>
          <DialogDescription>
            Modifiez le prompt pour une composition plus précise. Le guide de style
            de la campagne est déjà intégré au prompt par défaut.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          <div className="space-y-1">
            <Label htmlFor="img-prompt" className="text-xs uppercase">Prompt</Label>
            <Textarea
              id="img-prompt"
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              rows={5}
              className="font-mono text-xs"
            />
          </div>

          {previewUrl ? (
            <div className="space-y-1">
              <Label className="text-xs uppercase">Aperçu</Label>
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={previewUrl}
                alt="Image générée"
                className="rounded border w-full max-h-96 object-contain bg-black/30"
              />
            </div>
          ) : null}
        </div>

        <DialogFooter>
          {previewUrl ? (
            <>
              <Button variant="outline" onClick={generate} disabled={busy}>
                Régénérer
              </Button>
              <Button onClick={close}>Utiliser cette image</Button>
            </>
          ) : (
            <Button onClick={generate} disabled={busy}>
              {busy ? "Génération…" : "Générer"}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
