"use client";

import { useState, useRef, useCallback } from "react";
import { useRouter } from "next/navigation";
import { useQueryClient } from "@tanstack/react-query";
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
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";

const SAMPLE = `# Paste a single entity, multiple YAML docs separated by ---,
# or { entities: [...] } in one doc.
type: spell
name: Magic Missile
description: Three darts of force strike unerringly.
tags: [evocation, force, level 1]
attributes:
  level: 1
  school: evocation
  castingTime: 1 action
  range: 120 ft
  components: [V, S]
  duration: instantaneous
effects:
  - type: consume_resource
    resource: spell_slot
    amount: 1
    level: 1
    target:
      type: caster
  - type: damage
    amount: 3d4+3
    damageType: force
    target:
      type: single
      entityId: ""
visibility: public
`;

export function ImportYamlDialog({ campaignId }: { campaignId: string }) {
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const dropRef = useRef<HTMLDivElement>(null);
  const router = useRouter();
  const queryClient = useQueryClient();

  const onDrop = useCallback(async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const file = e.dataTransfer.files?.[0];
    if (!file) return;
    if (!/\.(yaml|yml|txt)$/i.test(file.name)) {
      toast.error("Drop a .yaml or .yml file");
      return;
    }
    const content = await file.text();
    setText(content);
  }, []);

  const onDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  async function submit() {
    if (!text.trim()) {
      toast.error("Paste or drop YAML first");
      return;
    }
    setBusy(true);
    try {
      const res = await fetch(`/api/campaigns/${campaignId}/entities/import`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ yaml: text }),
      });
      const data = await res.json();
      if (!res.ok) {
        const detail = data.error?.details
          ? JSON.stringify(data.error.details).slice(0, 200)
          : "";
        throw new Error(`${data.error?.message ?? "Import failed"}${detail ? ` — ${detail}` : ""}`);
      }
      toast.success(`Imported ${data.imported} entities`);
      setOpen(false);
      setText("");
      queryClient.invalidateQueries({ queryKey: ["entities", campaignId] });
      router.refresh();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Unknown error");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline">Import YAML</Button>
      </DialogTrigger>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Import entities from YAML</DialogTitle>
          <DialogDescription>
            Paste YAML or drop a .yaml file. Multiple entities are supported with
            <code className="mx-1 px-1 rounded bg-muted">---</code> separators or by
            using a top-level <code className="mx-1 px-1 rounded bg-muted">entities:</code> array.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          <div
            ref={dropRef}
            onDrop={onDrop}
            onDragOver={onDragOver}
            className="rounded border border-dashed p-2"
          >
            <Label htmlFor="yaml-input" className="text-xs uppercase tracking-wide text-muted-foreground">
              YAML content (drop a file here or paste below)
            </Label>
            <Textarea
              id="yaml-input"
              value={text}
              onChange={(e) => setText(e.target.value)}
              rows={14}
              className="font-mono text-xs"
              placeholder={SAMPLE}
            />
          </div>
        </div>

        <DialogFooter>
          <Button type="button" variant="ghost" onClick={() => setText(SAMPLE)}>
            Insert sample
          </Button>
          <Button type="button" onClick={submit} disabled={busy}>
            {busy ? "Importing…" : "Import"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
