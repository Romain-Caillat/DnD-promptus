"use client";

import { useState } from "react";
import * as yaml from "js-yaml";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { AlertTriangle, CircleX, CheckCircle2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { PHASE_LABELS } from "@/lib/engine/catalog";
import { MAP_LEVEL_LABELS, type CampaignStory } from "@/lib/engine/story";
import type { StoryIssue } from "@/lib/engine/story-validator";
import { skillLabel, abilityLabel } from "@/lib/engine/ruleset";
import { describeCondition } from "@/lib/story/describe";
import { MapLegend, MapView } from "@/components/maps/map-view";
import { useRuleset } from "@/components/providers/ruleset-provider";
import type { EntityRow } from "@/lib/db/schema";
import { cn } from "@/lib/utils";

interface StoryResponse {
  story: CampaignStory;
  issues: StoryIssue[];
}

export function storyQueryKey(campaignId: string) {
  return ["story", campaignId] as const;
}

export function StoryView({ campaignId }: { campaignId: string }) {
  const queryClient = useQueryClient();
  const { data, isLoading, error } = useQuery({
    queryKey: storyQueryKey(campaignId),
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/story`);
      if (!res.ok) throw new Error("Impossible de charger le scénario");
      return (await res.json()) as StoryResponse;
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

  if (error) return <p className="text-destructive">{(error as Error).message}</p>;
  if (isLoading || !data) return <p className="text-muted-foreground">Chargement du scénario…</p>;

  const names = new Map((entData?.entities ?? []).map((e) => [e.id, e.name]));
  return (
    <StoryDisplay
      story={data.story}
      issues={data.issues}
      entityName={(id) => names.get(id) ?? id}
      yamlEditor={
        <StoryYamlEditor
          story={data.story}
          saveUrl={`/api/campaigns/${campaignId}/story`}
          onSaved={(json) => queryClient.setQueryData(storyQueryKey(campaignId), json as StoryResponse)}
        />
      }
    />
  );
}

/** Affichage d'un scénario (campagne ou brouillon généré). */
export function StoryDisplay({
  story,
  issues,
  entityName,
  yamlEditor,
}: {
  story: CampaignStory;
  issues: StoryIssue[];
  entityName: (id: string) => string;
  yamlEditor: React.ReactNode;
}) {
  const ruleset = useRuleset();
  const sceneTitle = (id: string) => story.scenes.find((s) => s.id === id)?.title ?? id;
  const mapName = (id: string) => story.maps.find((m) => m.id === id)?.name ?? id;
  const errors = issues.filter((i) => i.severity === "error");
  const checkLabel = (c?: { skill?: string; ability?: string; dc?: number }) =>
    c
      ? [c.skill ? skillLabel(ruleset, c.skill) : c.ability ? abilityLabel(ruleset, c.ability) : null, c.dc ? `DD ${c.dc}` : null]
          .filter(Boolean)
          .join(" ")
      : "";

  return (
    <Tabs defaultValue="overview" className="space-y-4">
      <TabsList className="flex-wrap h-auto">
        <TabsTrigger value="overview">
          Vue d’ensemble
          {issues.length > 0 ? (
            <Badge variant={errors.length ? "destructive" : "secondary"} className="ml-1">
              {issues.length}
            </Badge>
          ) : null}
        </TabsTrigger>
        <TabsTrigger value="fronts">Menaces ({story.fronts.length})</TabsTrigger>
        <TabsTrigger value="scenes">Scènes ({story.scenes.length})</TabsTrigger>
        <TabsTrigger value="clues">Révélations ({story.revelations.length})</TabsTrigger>
        <TabsTrigger value="maps">Cartes ({story.maps.length})</TabsTrigger>
        <TabsTrigger value="yaml">Modifier (YAML)</TabsTrigger>
      </TabsList>

      <TabsContent value="overview" className="space-y-4">
        <IssuesPanel issues={issues} />
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Bible</CardTitle>
            {story.bible.tone ? <CardDescription>{story.bible.tone}</CardDescription> : null}
          </CardHeader>
          <CardContent className="space-y-3 text-sm">
            <p>{story.bible.pitch || <span className="italic text-muted-foreground">Pas encore de pitch.</span>}</p>
            {story.bible.playerHook ? (
              <p>
                <span className="font-medium">Accroche :</span> {story.bible.playerHook}
              </p>
            ) : null}
            {story.bible.startSceneId ? (
              <p>
                <span className="font-medium">Ouverture :</span> {sceneTitle(story.bible.startSceneId)}
              </p>
            ) : null}
            {story.bible.themes.length ? (
              <div className="flex flex-wrap gap-1">
                {story.bible.themes.map((t) => (
                  <Badge key={t} variant="secondary">
                    {t}
                  </Badge>
                ))}
              </div>
            ) : null}
            <ListBlock title="Vérités du monde" items={story.bible.truths} />
            <ListBlock title="Secrets (MJ)" items={story.bible.secrets} />
          </CardContent>
        </Card>
      </TabsContent>

      <TabsContent value="fronts" className="grid gap-4 md:grid-cols-2">
        {story.fronts.length === 0 ? <Empty text="Aucune menace." /> : null}
        {story.fronts.map((f) => (
          <Card key={f.id}>
            <CardHeader>
              <CardTitle className="text-base">{f.name}</CardTitle>
              <CardDescription>{f.goal}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-2 text-sm">
              {f.description ? <p className="text-muted-foreground">{f.description}</p> : null}
              <ol className="space-y-1">
                {f.steps.map((s, i) => (
                  <li key={i} className="flex gap-2">
                    <span
                      className={cn(
                        "mt-0.5 size-5 shrink-0 rounded-full border text-[10px] grid place-items-center",
                        i === f.steps.length - 1 && "border-destructive text-destructive",
                      )}
                    >
                      {i + 1}
                    </span>
                    <span>
                      <span className="font-medium">{s.label}</span>
                      {s.description ? ` : ${s.description}` : ""}
                    </span>
                  </li>
                ))}
              </ol>
            </CardContent>
          </Card>
        ))}
      </TabsContent>

      <TabsContent value="scenes" className="space-y-4">
        {story.scenes.length === 0 ? <Empty text="Aucune scène." /> : null}
        {story.scenes.map((s) => {
          const clues = story.clues.filter((c) => c.sceneId === s.id);
          const mapInfo = [
            s.mapPlacement ? `${mapName(s.mapPlacement.mapId)} (${s.mapPlacement.x}, ${s.mapPlacement.y})` : null,
            s.battleMapId ? `combat : ${mapName(s.battleMapId)}` : null,
          ]
            .filter(Boolean)
            .join(" · ");
          return (
            <Card key={s.id} id={`scene-${s.id}`}>
              <CardHeader>
                <div className="flex items-center gap-2 flex-wrap">
                  <CardTitle className="text-base">{s.title}</CardTitle>
                  <Badge variant="outline">{PHASE_LABELS[s.phase]}</Badge>
                  {story.bible.startSceneId === s.id ? <Badge>Ouverture</Badge> : null}
                  {s.exits.length === 0 ? <Badge variant="secondary">Finale</Badge> : null}
                </div>
                <CardDescription>{s.summary}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3 text-sm">
                {s.objective ? (
                  <p>
                    <span className="font-medium">Objectif :</span> {s.objective}
                  </p>
                ) : null}
                {s.readAloud ? (
                  <blockquote className="border-l-2 pl-3 italic text-muted-foreground">{s.readAloud}</blockquote>
                ) : null}
                {s.gmNotes ? (
                  <p>
                    <span className="font-medium">Notes MJ :</span> {s.gmNotes}
                  </p>
                ) : null}
                <div className="grid gap-2 md:grid-cols-2">
                  <Field label="Lieu" value={s.locationEntityId ? entityName(s.locationEntityId) : "—"} />
                  <Field label="PNJ" value={s.npcEntityIds.map(entityName).join(", ") || "—"} />
                  <Field label="Adversaires" value={s.monsterEntityIds.map(entityName).join(", ") || "—"} />
                  <Field label="Carte" value={mapInfo || "—"} />
                </div>
                {s.exits.length ? (
                  <Section title="Sorties">
                    {s.exits.map((x, i) => (
                      <li key={i}>
                        →{" "}
                        <a className="underline underline-offset-2" href={`#scene-${x.toSceneId}`}>
                          {sceneTitle(x.toSceneId)}
                        </a>
                        {x.label ? <span className="text-muted-foreground"> : {x.label}</span> : null}
                      </li>
                    ))}
                  </Section>
                ) : null}
                {clues.length ? (
                  <Section title="Indices ici">
                    {clues.map((c) => (
                      <li key={c.id}>
                        🔎 {c.discovery || c.text}
                        {c.check ? <span className="text-muted-foreground"> ({checkLabel(c.check)})</span> : null}
                      </li>
                    ))}
                  </Section>
                ) : null}
                {s.triggers.length ? (
                  <Section title="Déclencheurs">
                    {s.triggers.map((t) => (
                      <li key={t.id}>
                        ⚡ <span className="font-medium">{t.label}</span>
                        <span className="text-muted-foreground"> — quand {describeCondition(t.when, story, entityName)}</span>
                      </li>
                    ))}
                  </Section>
                ) : null}
              </CardContent>
            </Card>
          );
        })}
      </TabsContent>

      <TabsContent value="clues" className="space-y-4">
        {story.revelations.length === 0 ? <Empty text="Aucune révélation." /> : null}
        {story.revelations.map((r) => {
          const clues = story.clues.filter((c) => c.revelationId === r.id);
          const ok = r.importance === "optional" || clues.length >= 3;
          return (
            <Card key={r.id}>
              <CardHeader>
                <div className="flex items-center gap-2 flex-wrap">
                  <CardTitle className="text-base">{r.statement}</CardTitle>
                  <Badge variant={r.importance === "critical" ? "default" : "secondary"}>
                    {r.importance === "critical" ? "Essentielle" : "Facultative"}
                  </Badge>
                  <Badge variant={ok ? "outline" : "destructive"}>{clues.length} indice(s)</Badge>
                </div>
              </CardHeader>
              <CardContent>
                <ul className="space-y-2 text-sm">
                  {clues.map((c) => (
                    <li key={c.id}>
                      <p>{c.text}</p>
                      <p className="text-xs text-muted-foreground">
                        {sceneTitle(c.sceneId)} · {c.discovery}
                        {c.check ? ` · ${checkLabel(c.check)}` : ""}
                      </p>
                    </li>
                  ))}
                </ul>
              </CardContent>
            </Card>
          );
        })}
      </TabsContent>

      <TabsContent value="maps" className="space-y-4">
        {story.maps.length === 0 ? <Empty text="Aucune carte." /> : null}
        {story.maps.length ? <MapLegend /> : null}
        {story.maps.map((m) => {
          const blocked = m.cells.filter((c) => c.blocked).length;
          const children = m.cells.filter((c) => c.childMapId);
          const scenesHere = story.scenes.filter((s) => s.mapPlacement?.mapId === m.id || s.battleMapId === m.id);
          return (
            <Card key={m.id} data-testid={`story-map-${m.id}`}>
              <CardHeader>
                <div className="flex items-center gap-2 flex-wrap">
                  <CardTitle className="text-base">{m.name}</CardTitle>
                  <Badge variant="outline">{MAP_LEVEL_LABELS[m.level]}</Badge>
                </div>
                <CardDescription>
                  Grille {m.grid.type === "hex" ? "hexagonale" : "carrée"} {m.grid.cols} × {m.grid.rows}
                  {blocked ? ` · ${blocked} case(s) infranchissable(s)` : ""}
                  {m.tokens?.length ? ` · ${m.tokens.length} pion(s)` : ""}
                  {scenesHere.length ? ` · scènes : ${scenesHere.map((s) => s.title).join(", ")}` : ""}
                  {children.length ? ` · ouvre : ${children.map((c) => mapName(c.childMapId!)).join(", ")}` : ""}
                </CardDescription>
              </CardHeader>
              <CardContent>
                <MapView
                  map={m}
                  tokens={m.tokens ?? []}
                  tokenInfo={(id) => {
                    const name = entityName(id);
                    return { label: name.slice(0, 2).toUpperCase(), kind: "enemy", title: name };
                  }}
                />
              </CardContent>
            </Card>
          );
        })}
      </TabsContent>

      <TabsContent value="yaml">{yamlEditor}</TabsContent>
    </Tabs>
  );
}

function IssuesPanel({ issues }: { issues: StoryIssue[] }) {
  if (issues.length === 0) {
    return (
      <Card>
        <CardContent className="pt-4 flex items-center gap-2 text-sm">
          <CheckCircle2 className="size-4 text-emerald-500" /> Aucun problème détecté : références valides,
          indices suffisants, scènes accessibles.
        </CardContent>
      </Card>
    );
  }
  return (
    <Card data-testid="story-issues">
      <CardHeader>
        <CardTitle className="text-base">Problèmes détectés</CardTitle>
      </CardHeader>
      <CardContent>
        <ul className="space-y-1 text-sm">
          {issues.map((i, k) => (
            <li key={k} className="flex gap-2">
              {i.severity === "error" ? (
                <CircleX className="size-4 shrink-0 text-destructive mt-0.5" />
              ) : (
                <AlertTriangle className="size-4 shrink-0 text-amber-500 mt-0.5" />
              )}
              <span>
                {i.message} <span className="text-xs text-muted-foreground font-mono">{i.path}</span>
              </span>
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}

export function StoryYamlEditor({
  story,
  saveUrl,
  onSaved,
}: {
  story: CampaignStory;
  /** PUT { story } ; la réponse doit contenir `issues` ou `job.result.issues`. */
  saveUrl: string;
  onSaved: (json: unknown) => void;
}) {
  const [draft, setDraft] = useState<string | null>(null);
  const [problems, setProblems] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const text = draft ?? yaml.dump(story, { lineWidth: 120, noRefs: true });

  async function save() {
    let parsed: unknown;
    try {
      parsed = yaml.load(text);
    } catch (e) {
      setProblems([`YAML invalide : ${e instanceof Error ? e.message : String(e)}`]);
      return;
    }
    setBusy(true);
    setProblems([]);
    try {
      const res = await fetch(saveUrl, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ story: parsed }),
      });
      const json = await res.json();
      if (!res.ok) {
        const details = (json.error?.details ?? []) as { path: (string | number)[]; message: string }[];
        // Les erreurs de validation s'affichent sous l'éditeur, pas en toast
        // (qui masquerait le bouton d'enregistrement).
        if (details.length > 0) {
          setProblems(details.map((d) => `${d.path.join(".") || "(racine)"} : ${d.message}`));
          return;
        }
        throw new Error(json.error?.message ?? "Échec de l’enregistrement");
      }
      onSaved(json);
      setDraft(null);
      const warnings = (json.issues ?? json.job?.result?.issues ?? []).length;
      toast.success(warnings ? `Scénario enregistré (${warnings} avertissement(s))` : "Scénario enregistré");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Erreur inconnue");
    } finally {
      setBusy(false);
    }
  }

  function download() {
    const blob = new Blob([text], { type: "text/yaml" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = "scenario.yaml";
    a.click();
    URL.revokeObjectURL(a.href);
  }

  return (
    <Card>
      <CardContent className="pt-4 space-y-3">
        <p className="text-sm text-muted-foreground">
          Le scénario complet au format YAML : c’est aussi le format que produira la génération par IA. Les erreurs
          (références cassées) bloquent l’enregistrement ; les avertissements non.
        </p>
        <Textarea
          value={text}
          onChange={(e) => setDraft(e.target.value)}
          className="font-mono text-xs h-[60vh] field-sizing-fixed overflow-auto"
          spellCheck={false}
          aria-label="Scénario au format YAML"
        />
        {problems.length > 0 ? (
          <ul className="text-sm text-destructive list-disc pl-5" data-testid="story-problems">
            {problems.map((p) => (
              <li key={p}>{p}</li>
            ))}
          </ul>
        ) : null}
        <div className="flex flex-wrap justify-between gap-2">
          <Button variant="outline" onClick={download}>
            Télécharger le YAML
          </Button>
          <div className="flex gap-2">
            <Button
              variant="ghost"
              disabled={busy || draft === null}
              onClick={() => {
                setDraft(null);
                setProblems([]);
              }}
            >
              Annuler les modifications
            </Button>
            <Button disabled={busy || draft === null} onClick={save}>
              {busy ? "Enregistrement…" : "Enregistrer"}
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function ListBlock({ title, items }: { title: string; items: string[] }) {
  if (!items.length) return null;
  return (
    <Section title={title} className="list-disc pl-5">
      {items.map((t) => (
        <li key={t}>{t}</li>
      ))}
    </Section>
  );
}

function Section({ title, children, className }: { title: string; children: React.ReactNode; className?: string }) {
  return (
    <div>
      <p className="text-xs uppercase tracking-wide text-muted-foreground mb-1">{title}</p>
      <ul className={cn("space-y-0.5", className)}>{children}</ul>
    </div>
  );
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <p>
      <span className="text-xs uppercase tracking-wide text-muted-foreground">{label} : </span>
      {value}
    </p>
  );
}

function Empty({ text }: { text: string }) {
  return <p className="text-sm italic text-muted-foreground">{text}</p>;
}
