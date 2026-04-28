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
    if (!entityId) return toast.error("Save the entity first to generate an image");
    if (!prompt.trim()) return toast.error("Prompt cannot be empty");
    setBusy(true);
    try {
      const res = await fetch(`/api/ai/image`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ entityId, prompt: prompt.trim() }),
      });
      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error?.message ?? "Generation failed");
      }
      setPreviewUrl(data.result.url);
      onGenerated(data.result.url);
      toast.success(`Image generated (${data.result.latencyMs}ms)`);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Generation failed");
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
          title={!entityId ? "Save the entity first" : undefined}
        >
          <Sparkles className="size-4" /> Generate
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>Generate image</DialogTitle>
          <DialogDescription>
            Edit the prompt if you want a more specific composition. The campaign
            style guide is already factored into the default prompt.
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
              <Label className="text-xs uppercase">Preview</Label>
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={previewUrl}
                alt="Generated"
                className="rounded border w-full max-h-96 object-contain bg-black/30"
              />
            </div>
          ) : null}
        </div>

        <DialogFooter>
          {previewUrl ? (
            <>
              <Button variant="outline" onClick={generate} disabled={busy}>
                Regenerate
              </Button>
              <Button onClick={close}>Use this image</Button>
            </>
          ) : (
            <Button onClick={generate} disabled={busy}>
              {busy ? "Generating…" : "Generate"}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
