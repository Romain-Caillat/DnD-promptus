"use client";

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { BookOpen, CheckCircle2, Hourglass, Search, UserPlus, Zap } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { PHASE_LABELS } from "@/lib/engine/catalog";
import { abilityLabel, skillLabel } from "@/lib/engine/ruleset";
import type { CampaignStory } from "@/lib/engine/story";
import type { Effect } from "@/lib/engine/types";
import {
  frontStep,
  isRevelationKnown,
  sceneTriggers,
  type WorldState,
} from "@/lib/engine/world";
import { describeCondition } from "@/lib/story/describe";
import { useRuleset } from "@/components/providers/ruleset-provider";
import type { ParticipantView } from "@/lib/stores/session-store";
import type { EntityRow } from "@/lib/db/schema";
import { cn } from "@/lib/utils";

interface StoryData {
  story: CampaignStory;
  world: WorldState;
}

export function StoryPanel({
  sessionId,
  campaignId,
  participants,
}: {
  sessionId: string;
  campaignId: string;
  participants: ParticipantView[];
}) {
  const ruleset = useRuleset();
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [picked, setPicked] = useState("");

  const { data } = useQuery({
    queryKey: ["session-story", sessionId],
    queryFn: async () => {
      const res = await fetch(`/api/sessions/${sessionId}/story`);
      if (!res.ok) throw new Error("Impossible de charger le scénario");
      return (await res.json()) as StoryData;
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
  const entityName = (id: string) => names.get(id) ?? id;

  if (!data) return null;
  const { story, world } = data;
  if (story.scenes.length === 0) {
    return (
      <Card>
        <CardContent className="pt-4 text-sm text-muted-foreground italic">
          Cette campagne n’a pas encore de scénario (page Scénario de la campagne).
        </CardContent>
      </Card>
    );
  }

  const scene = story.scenes.find((s) => s.id === world.currentSceneId);
  const sceneTitle = (id: string) => story.scenes.find((s) => s.id === id)?.title ?? id;
  const triggers = sceneTriggers(story, world);
  const present = new Set(participants.map((p) => p.state.entityId));
  const sceneCast = scene ? [...scene.npcEntityIds, ...scene.monsterEntityIds] : [];
  const missingCast = sceneCast.filter((id) => !present.has(id));

  function refresh() {
    queryClient.invalidateQueries({ queryKey: ["session-story", sessionId] });
    queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    queryClient.invalidateQueries({ queryKey: ["timeline", sessionId] });
  }

  async function act(body: object, ok?: string) {
    setBusy(true);
    try {
      const res = await fetch(`/api/sessions/${sessionId}/actions`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      });
      if (!res.ok) throw new Error((await res.json()).error?.message ?? "Action impossible");
      if (ok) toast.success(ok);
      refresh();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Action impossible");
    } finally {
      setBusy(false);
    }
  }

  const effects = (...list: Effect[]) => act({ kind: "raw_effects", effects: list });

  async function addCast() {
    setBusy(true);
    try {
      const res = await fetch(`/api/sessions/${sessionId}/participants`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ entityIds: missingCast }),
      });
      if (!res.ok) throw new Error((await res.json()).error?.message ?? "Ajout impossible");
      toast.success(`${missingCast.length} participant(s) ajouté(s)`);
      refresh();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Ajout impossible");
    } finally {
      setBusy(false);
    }
  }

  const checkLabel = (c?: { skill?: string; ability?: string; dc?: number }) =>
    c
      ? [c.skill ? skillLabel(ruleset, c.skill) : c.ability ? abilityLabel(ruleset, c.ability) : null, c.dc ? `DD ${c.dc}` : null]
          .filter(Boolean)
          .join(" ")
      : "";

  return (
    <Card data-testid="story-panel">
      <CardContent className="pt-4 space-y-4">
        <div className="flex items-center justify-between gap-2 flex-wrap">
          <div className="flex items-center gap-2">
            <BookOpen className="size-4 text-muted-foreground" />
            <h2 className="text-sm font-semibold uppercase tracking-wide">Scénario</h2>
          </div>
          <div className="flex items-center gap-2">
            <Select value={picked} onValueChange={setPicked}>
              <SelectTrigger className="w-64 h-8 text-xs" aria-label="Choisir une scène">
                <SelectValue placeholder="Aller à une scène…" />
              </SelectTrigger>
              <SelectContent>
                {story.scenes.map((s) => (
                  <SelectItem key={s.id} value={s.id}>
                    {s.title}
                    {world.sceneStatus[s.id] ? ` (${world.sceneStatus[s.id] === "resolved" ? "résolue" : "visitée"})` : ""}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Button
              size="sm"
              disabled={busy || !picked}
              onClick={() => {
                void effects({ type: "enter_scene", sceneId: picked });
                setPicked("");
              }}
            >
              Entrer
            </Button>
          </div>
        </div>

        {!scene ? (
          <div className="rounded border border-dashed p-4 text-sm space-y-2">
            <p className="text-muted-foreground">Aucune scène en cours.</p>
            {story.bible.startSceneId ? (
              <Button
                disabled={busy}
                onClick={() => effects({ type: "enter_scene", sceneId: story.bible.startSceneId! })}
              >
                Commencer : {sceneTitle(story.bible.startSceneId)}
              </Button>
            ) : null}
          </div>
        ) : (
          <div className="space-y-4">
            <div className="flex items-center gap-2 flex-wrap">
              <h3 className="text-xl font-semibold" data-testid="current-scene">{scene.title}</h3>
              <Badge variant="outline">{PHASE_LABELS[scene.phase]}</Badge>
              {world.sceneStatus[scene.id] === "resolved" ? <Badge>Résolue</Badge> : null}
            </div>
            {scene.readAloud ? (
              <blockquote className="border-l-4 border-primary/60 pl-4 text-base leading-relaxed">
                {scene.readAloud}
              </blockquote>
            ) : null}
            <div className="grid gap-1 text-sm md:grid-cols-2">
              <p><span className="text-muted-foreground">Objectif : </span>{scene.objective || "—"}</p>
              <p><span className="text-muted-foreground">Résumé MJ : </span>{scene.summary || "—"}</p>
              {scene.npcEntityIds.length ? (
                <p><span className="text-muted-foreground">PNJ : </span>{scene.npcEntityIds.map(entityName).join(", ")}</p>
              ) : null}
              {scene.monsterEntityIds.length ? (
                <p><span className="text-muted-foreground">Adversaires : </span>{scene.monsterEntityIds.map(entityName).join(", ")}</p>
              ) : null}
              {scene.gmNotes ? <p className="md:col-span-2"><span className="text-muted-foreground">Notes : </span>{scene.gmNotes}</p> : null}
            </div>

            {missingCast.length ? (
              <Button size="sm" variant="outline" disabled={busy} onClick={addCast}>
                <UserPlus className="size-4" /> Ajouter à la session : {missingCast.map(entityName).join(", ")}
              </Button>
            ) : null}

            {/* Déclencheurs : proposés, jamais appliqués sans le MJ */}
            {triggers.length ? (
              <div className="space-y-1">
                <p className="text-xs uppercase tracking-wide text-muted-foreground">Déclencheurs</p>
                {triggers.map(({ trigger, ready, fired }) => (
                  <div
                    key={trigger.id}
                    className={cn(
                      "flex items-center justify-between gap-2 rounded border px-3 py-2 text-sm",
                      ready ? "border-amber-500 bg-amber-500/10" : "opacity-60",
                    )}
                    data-testid={ready ? "trigger-ready" : "trigger-waiting"}
                  >
                    <span>
                      <Zap className={cn("inline size-4 mr-1", ready && "text-amber-500")} />
                      <span className="font-medium">{trigger.label}</span>
                      <span className="text-muted-foreground">
                        {fired ? " — déjà déclenché" : ` — quand ${describeCondition(trigger.when, story, entityName)}`}
                      </span>
                    </span>
                    {ready ? (
                      <Button
                        size="sm"
                        disabled={busy}
                        onClick={() => act({ kind: "fire_trigger", sceneId: scene.id, triggerId: trigger.id })}
                      >
                        Déclencher
                      </Button>
                    ) : null}
                  </div>
                ))}
              </div>
            ) : null}

            {/* Indices de la scène */}
            {story.clues.some((c) => c.sceneId === scene.id) ? (
              <div className="space-y-1">
                <p className="text-xs uppercase tracking-wide text-muted-foreground">Indices</p>
                {story.clues
                  .filter((c) => c.sceneId === scene.id)
                  .map((c) => {
                    const found = world.foundClueIds.includes(c.id);
                    return (
                      <div key={c.id} className="flex items-center justify-between gap-2 text-sm">
                        <span className={cn(found && "text-muted-foreground line-through")}>
                          <Search className="inline size-3.5 mr-1" />
                          {c.discovery || c.text}
                          {c.check ? <span className="text-muted-foreground"> ({checkLabel(c.check)})</span> : null}
                          {!found ? <span className="block text-xs text-muted-foreground pl-5">{c.text}</span> : null}
                        </span>
                        {found ? (
                          <CheckCircle2 className="size-4 text-emerald-500 shrink-0" />
                        ) : (
                          <Button
                            size="sm"
                            variant="outline"
                            disabled={busy}
                            onClick={() => effects({ type: "reveal_clue", clueId: c.id })}
                          >
                            Trouvé
                          </Button>
                        )}
                      </div>
                    );
                  })}
              </div>
            ) : null}

            {/* Sorties */}
            <div className="flex flex-wrap gap-2">
              {scene.exits.map((x) => (
                <Button
                  key={x.toSceneId}
                  variant="secondary"
                  size="sm"
                  disabled={busy}
                  title={x.label}
                  onClick={() => effects({ type: "enter_scene", sceneId: x.toSceneId })}
                >
                  → {sceneTitle(x.toSceneId)}
                </Button>
              ))}
              {world.sceneStatus[scene.id] !== "resolved" ? (
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={busy}
                  onClick={() => effects({ type: "set_scene_status", sceneId: scene.id, status: "resolved" })}
                >
                  Marquer la scène résolue
                </Button>
              ) : null}
            </div>
          </div>
        )}

        {/* Menaces */}
        {story.fronts.length ? (
          <div className="grid gap-2 md:grid-cols-2 border-t pt-3">
            {story.fronts.map((f) => {
              const step = frontStep(world, f.id);
              return (
                <div key={f.id} className="text-sm space-y-1" data-testid={`front-${f.id}`}>
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-medium">
                      <Hourglass className="inline size-3.5 mr-1" />
                      {f.name}
                    </span>
                    <div className="flex gap-1">
                      <Button size="xs" variant="ghost" disabled={busy || step < 0} aria-label={`Reculer ${f.name}`}
                        onClick={() => effects({ type: "advance_front", frontId: f.id, steps: -1 })}>−</Button>
                      <Button size="xs" variant="outline" disabled={busy || step >= f.steps.length - 1} aria-label={`Avancer ${f.name}`}
                        onClick={() => effects({ type: "advance_front", frontId: f.id, steps: 1 })}>+1</Button>
                    </div>
                  </div>
                  <div className="flex gap-1" aria-hidden>
                    {f.steps.map((_, i) => (
                      <span
                        key={i}
                        className={cn(
                          "h-1.5 flex-1 rounded-full bg-muted",
                          i <= step && (i === f.steps.length - 1 ? "bg-destructive" : "bg-amber-500"),
                        )}
                      />
                    ))}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {step < 0 ? "Pas encore commencée" : `${f.steps[step].label} : ${f.steps[step].description}`}
                  </p>
                </div>
              );
            })}
          </div>
        ) : null}

        {story.revelations.length ? (
          <p className="text-xs text-muted-foreground border-t pt-3">
            Révélations connues :{" "}
            {story.revelations.filter((r) => isRevelationKnown(story, world, r.id)).map((r) => `« ${r.statement} »`).join(", ") || "aucune"}
          </p>
        ) : null}
      </CardContent>
    </Card>
  );
}
