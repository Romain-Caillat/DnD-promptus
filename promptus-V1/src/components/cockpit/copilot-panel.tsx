"use client";

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Check, MessageSquareQuote, Send, Sparkles } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { COPILOT_KIND_LABELS, type CopilotAction, type CopilotKind } from "@/lib/copilot/copilot";
import type { CopilotCallView } from "@/lib/copilot/server";
import type { EntityRow } from "@/lib/db/schema";
import type { CampaignStory } from "@/lib/engine/story";
import type { WorldState } from "@/lib/engine/world";
import type { ParticipantView } from "@/lib/stores/session-store";

interface Props {
  sessionId: string;
  campaignId: string;
  participants: ParticipantView[];
}

async function postJson(url: string, body: unknown) {
  const res = await fetch(url, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
  const json = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(json.error?.message ?? "Échec");
  return json;
}

/** Libellé lisible d'une action proposée. */
function actionLabel(a: CopilotAction, story: CampaignStory | undefined, names: Map<string, string>): string {
  switch (a.type) {
    case "fire_trigger": {
      const t = story?.scenes.find((s) => s.id === a.sceneId)?.triggers.find((x) => x.id === a.triggerId);
      return `Déclencher « ${t?.label ?? a.triggerId} »`;
    }
    case "reveal_clue":
      return `Révéler l’indice « ${story?.clues.find((c) => c.id === a.clueId)?.discovery ?? a.clueId} »`;
    case "advance_front":
      return `Faire avancer « ${story?.fronts.find((f) => f.id === a.frontId)?.name ?? a.frontId} »`;
    case "enter_scene":
      return `Aller à « ${story?.scenes.find((s) => s.id === a.sceneId)?.title ?? a.sceneId} »`;
    case "reveal_entity":
      return `Révéler ${names.get(a.entityId) ?? a.entityId}`;
  }
}

export function CopilotPanel({ sessionId, campaignId, participants }: Props) {
  const queryClient = useQueryClient();
  const [prompt, setPrompt] = useState("");
  const [npcId, setNpcId] = useState("");
  /** Textes modifiés par le MJ avant envoi, et éléments déjà utilisés. */
  const [edits, setEdits] = useState<Record<string, string>>({});
  const [used, setUsed] = useState<Set<string>>(new Set());
  const markUsed = (k: string) => setUsed((s) => new Set(s).add(k));

  const key = ["copilot", sessionId];
  const { data } = useQuery({
    queryKey: key,
    queryFn: async () => {
      const res = await fetch(`/api/sessions/${sessionId}/copilot`);
      if (!res.ok) throw new Error("Impossible de charger le co-MJ");
      return (await res.json()) as { configured: boolean; calls: CopilotCallView[] };
    },
  });
  const { data: storyData } = useQuery({
    queryKey: ["session-story", sessionId],
    queryFn: async () => {
      const res = await fetch(`/api/sessions/${sessionId}/story`);
      if (!res.ok) throw new Error("Impossible de charger le scénario");
      return (await res.json()) as { story: CampaignStory; world: WorldState };
    },
  });

  const ask = useMutation({
    mutationFn: (kind: CopilotKind) =>
      postJson(`/api/sessions/${sessionId}/copilot`, {
        kind,
        npcId: kind === "npc" ? npcId || undefined : undefined,
        prompt: prompt || undefined,
      }),
    onSuccess: () => {
      setPrompt("");
      void queryClient.invalidateQueries({ queryKey: key });
    },
    onError: (e) => {
      toast.error(e.message);
      void queryClient.invalidateQueries({ queryKey: key });
    },
  });

  const { data: entData } = useQuery({
    queryKey: ["entities", campaignId, "all"],
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/entities`);
      if (!res.ok) throw new Error("Impossible de charger les fiches");
      return (await res.json()) as { entities: EntityRow[] };
    },
  });
  const names = new Map((entData?.entities ?? []).map((e) => [e.id, e.name]));
  const scene = storyData?.story.scenes.find((s) => s.id === storyData.world.currentSceneId);
  // Qui peut parler : PNJ et créatures de la scène, puis participants non joueurs.
  const npcIds = [...new Set([...(scene?.npcEntityIds ?? []), ...(scene?.monsterEntityIds ?? []), ...participants.filter((p) => p.entity && p.entity.type !== "character").map((p) => p.state.entityId)])];

  async function sendToPlayers(k: string, text: string, speaker?: string) {
    try {
      await postJson(`/api/sessions/${sessionId}/narration`, { text, speaker });
      markUsed(k);
      void queryClient.invalidateQueries({ queryKey: ["timeline", sessionId] });
      toast.success("Envoyé aux joueurs");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Échec");
    }
  }

  async function apply(k: string, a: CopilotAction) {
    try {
      const body =
        a.type === "fire_trigger"
          ? { kind: "fire_trigger", sceneId: a.sceneId, triggerId: a.triggerId }
          : {
              kind: "raw_effects",
              effects: [a.type === "reveal_entity" ? { type: "reveal_entity", entityId: a.entityId, toUsers: "all_players" } : a],
            };
      await postJson(`/api/sessions/${sessionId}/actions`, body);
      markUsed(k);
      for (const q of [["session", sessionId], ["session-story", sessionId], ["timeline", sessionId]]) {
        void queryClient.invalidateQueries({ queryKey: q });
      }
      toast.success("Appliqué");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Échec");
    }
  }

  const calls = data?.calls ?? [];
  const [latest, ...older] = calls;

  function renderCall(c: CopilotCallView, full: boolean) {
    const head = (
      <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
        <span>
          {COPILOT_KIND_LABELS[c.request.kind]}
          {c.request.prompt ? ` — « ${c.request.prompt} »` : ""}
        </span>
        <span>{c.costUsd > 0 ? `${c.costUsd.toFixed(3)} $` : ""}</span>
      </div>
    );
    if (c.status === "failed") {
      return (
        <div key={c.id} className="rounded border border-destructive/50 p-2 space-y-1">
          {head}
          <p className="text-xs text-destructive">{c.error}</p>
        </div>
      );
    }
    if (!c.answer) return null;
    const a = c.answer;
    if (!full) {
      return (
        <details key={c.id} className="rounded border p-2 text-xs">
          <summary className="cursor-pointer">{head}</summary>
          <div className="mt-1 space-y-1 whitespace-pre-line">
            {a.narration ? <p>{a.narration}</p> : null}
            {a.npcLines.map((l, i) => (
              <p key={i}>
                <b>{l.speaker}</b> : « {l.text} »
              </p>
            ))}
            {a.suggestions.map((s, i) => (
              <p key={i}>→ {s.label}</p>
            ))}
          </div>
        </details>
      );
    }
    const nk = `${c.id}:narration`;
    return (
      <div key={c.id} className="rounded border p-3 space-y-3" data-testid="copilot-answer">
        {head}
        {a.narration ? (
          <div className="space-y-1">
            <Textarea
              aria-label="Narration proposée"
              value={edits[nk] ?? a.narration}
              onChange={(e) => setEdits((d) => ({ ...d, [nk]: e.target.value }))}
              rows={4}
              className="text-sm"
            />
            <Button size="xs" disabled={used.has(nk)} onClick={() => sendToPlayers(nk, edits[nk] ?? a.narration)}>
              {used.has(nk) ? <Check /> : <Send />} {used.has(nk) ? "Envoyé" : "Envoyer aux joueurs"}
            </Button>
          </div>
        ) : null}
        {a.npcLines.map((l, i) => {
          const k = `${c.id}:npc:${i}`;
          return (
            <div key={k} className="space-y-1">
              <p className="text-xs font-medium flex items-center gap-1">
                <MessageSquareQuote className="size-3" /> {l.speaker}
              </p>
              <Textarea
                aria-label={`Réplique de ${l.speaker}`}
                value={edits[k] ?? l.text}
                onChange={(e) => setEdits((d) => ({ ...d, [k]: e.target.value }))}
                rows={2}
                className="text-sm"
              />
              <Button size="xs" variant="outline" disabled={used.has(k)} onClick={() => sendToPlayers(k, edits[k] ?? l.text, l.speaker)}>
                {used.has(k) ? <Check /> : <Send />} {used.has(k) ? "Envoyé" : "Envoyer la réplique"}
              </Button>
            </div>
          );
        })}
        {a.suggestions.length ? (
          <ul className="space-y-1">
            {a.suggestions.map((s, i) => {
              const k = `${c.id}:sugg:${i}`;
              return (
                <li key={k} className="flex items-start justify-between gap-2 text-sm">
                  <div>
                    <p>→ {s.label}</p>
                    {s.why ? <p className="text-xs text-muted-foreground">{s.why}</p> : null}
                  </div>
                  {s.action ? (
                    <Button size="xs" variant="outline" disabled={used.has(k)} onClick={() => apply(k, s.action!)} title={actionLabel(s.action, storyData?.story, names)}>
                      {used.has(k) ? <Check /> : null} {used.has(k) ? "Fait" : actionLabel(s.action, storyData?.story, names)}
                    </Button>
                  ) : null}
                </li>
              );
            })}
          </ul>
        ) : null}
        {a.gmNote ? <p className="text-xs italic text-muted-foreground">MJ : {a.gmNote}</p> : null}
      </div>
    );
  }

  return (
    <Card data-testid="copilot-panel">
      <CardContent className="pt-4 space-y-3">
        <div className="flex items-center justify-between gap-2">
          <h2 className="text-sm font-semibold uppercase tracking-wide flex items-center gap-1">
            <Sparkles className="size-4" /> Co-MJ
          </h2>
          <span className="text-xs text-muted-foreground">Il propose, vous validez. Rien n’est montré sans votre accord.</span>
        </div>
        {data && !data.configured ? (
          <p className="text-xs text-muted-foreground italic">
            Co-MJ indisponible : ajoutez OPENROUTER_API_KEY dans la configuration du serveur.
          </p>
        ) : (
          <>
            <Input
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="Précision facultative : ce que disent ou tentent les joueurs, votre question…"
              aria-label="Précision pour le co-MJ"
              onKeyDown={(e) => {
                if (e.key === "Enter" && prompt.trim() && !ask.isPending) ask.mutate("free");
              }}
            />
            <div className="flex flex-wrap items-center gap-1">
              {(["describe", "consequence", "next"] as const).map((k) => (
                <Button key={k} size="xs" variant="outline" disabled={ask.isPending} onClick={() => ask.mutate(k)}>
                  {COPILOT_KIND_LABELS[k]}
                </Button>
              ))}
              <div className="flex items-center gap-1">
                <Select value={npcId} onValueChange={setNpcId}>
                  <SelectTrigger className="h-7 w-40 text-xs" aria-label="PNJ qui parle">
                    <SelectValue placeholder="PNJ…" />
                  </SelectTrigger>
                  <SelectContent>
                    {npcIds.map((id) => (
                      <SelectItem key={id} value={id}>
                        {names.get(id) ?? id}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Button size="xs" variant="outline" disabled={ask.isPending || !npcId} onClick={() => ask.mutate("npc")}>
                  Faire parler
                </Button>
              </div>
              <Button size="xs" disabled={ask.isPending || !prompt.trim()} onClick={() => ask.mutate("free")}>
                Demander
              </Button>
            </div>
            {ask.isPending ? (
              <p className="text-xs text-muted-foreground animate-pulse" data-testid="copilot-thinking">
                Le co-MJ réfléchit…
              </p>
            ) : null}
            {latest ? renderCall(latest, true) : null}
            {older.length ? (
              <div className="space-y-1">
                <Badge variant="outline" className="text-xs">
                  Propositions précédentes
                </Badge>
                {older.slice(0, 5).map((c) => renderCall(c, false))}
              </div>
            ) : null}
          </>
        )}
      </CardContent>
    </Card>
  );
}
