"use client";

import { useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { CheckCircle2, CircleDashed, CircleX, Loader2, Sparkles } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
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
import { ENTITY_TYPE_LABELS } from "@/lib/engine/catalog";
import { LENGTH_LABELS, type GenerationInput, type GenerationJobView } from "@/lib/generation/types";
import { StoryDisplay, StoryYamlEditor } from "@/components/story/story-view";
import { cn } from "@/lib/utils";

interface StudioStatus {
  configured: boolean;
  model: string;
  budgetUsd: number | null;
  spentUsd: number;
  hasStory: boolean;
  jobs: GenerationJobView[];
}

const usd = (n: number) => `${n.toFixed(n < 1 ? 3 : 2)} $`;

const STATUS_LABELS: Record<GenerationJobView["status"], string> = {
  running: "en cours",
  succeeded: "prêt à relire",
  failed: "échec",
  applied: "appliqué",
};

export function GenerationStudio({ campaignId }: { campaignId: string }) {
  const queryClient = useQueryClient();
  const [jobId, setJobId] = useState<string | null>(null);

  const statusKey = ["generation-status", campaignId];
  const { data: status } = useQuery({
    queryKey: statusKey,
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/generate`);
      if (!res.ok) throw new Error("Impossible de charger l’état de la génération");
      return (await res.json()) as StudioStatus;
    },
    // Dépense et historique à jour tant qu'une génération tourne.
    refetchInterval: (q) => (q.state.data?.jobs.some((j) => j.status === "running") ? 3000 : false),
  });

  // Job affiché : celui choisi, sinon le plus récent.
  const shownId = jobId ?? status?.jobs[0]?.id ?? null;
  const { data: jobData } = useQuery({
    queryKey: ["generation-job", shownId],
    enabled: !!shownId,
    queryFn: async () => {
      const res = await fetch(`/api/generation-jobs/${shownId}`);
      if (!res.ok) throw new Error("Impossible de charger la génération");
      return (await res.json()) as { job: GenerationJobView };
    },
    // Suivi en direct tant que le job tourne.
    refetchInterval: (q) => (q.state.data?.job.status === "running" ? 2000 : false),
  });
  const job = jobData?.job ?? null;

  if (!status) return <p className="text-muted-foreground">Chargement…</p>;

  const budgetLeft = status.budgetUsd === null ? null : Math.max(0, status.budgetUsd - status.spentUsd);

  return (
    <div className="space-y-6">
      {!status.configured ? (
        <Card className="border-amber-500">
          <CardContent className="pt-4 text-sm space-y-1">
            <p className="font-medium">La génération n’est pas configurée sur ce serveur.</p>
            <p className="text-muted-foreground">
              Ajoutez <code>OPENROUTER_API_KEY</code> (clé créée sur openrouter.ai) dans l’environnement du serveur
              (<code>.env.local</code> en local), puis redémarrez l’application.
            </p>
          </CardContent>
        </Card>
      ) : null}

      <GenerationForm
        campaignId={campaignId}
        disabled={!status.configured || job?.status === "running" || budgetLeft === 0}
        status={status}
        onStarted={(id) => {
          setJobId(id);
          void queryClient.invalidateQueries({ queryKey: statusKey });
        }}
      />

      {job ? (
        <JobPanel
          job={job}
          hasStory={status.hasStory}
          onChanged={() => {
            void queryClient.invalidateQueries({ queryKey: statusKey });
            void queryClient.invalidateQueries({ queryKey: ["generation-job", job.id] });
          }}
        />
      ) : null}

      {status.jobs.length > 1 ? (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Générations précédentes</CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-1 text-sm">
              {status.jobs.map((j) => (
                <li key={j.id}>
                  <button
                    type="button"
                    onClick={() => setJobId(j.id)}
                    className={cn("text-left hover:underline", j.id === shownId && "font-medium")}
                  >
                    {new Date(j.createdAt).toLocaleString("fr-FR")} · {STATUS_LABELS[j.status]} · {usd(j.usage.costUsd)} ·{" "}
                    <span className="text-muted-foreground">{j.input.pitch.slice(0, 80)}</span>
                  </button>
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}

function GenerationForm({
  campaignId,
  disabled,
  status,
  onStarted,
}: {
  campaignId: string;
  disabled: boolean;
  status: StudioStatus;
  onStarted: (jobId: string) => void;
}) {
  const [form, setForm] = useState({
    pitch: "",
    tone: "",
    themes: "",
    players: "4",
    level: "1",
    length: "one_shot" as GenerationInput["length"],
    constraints: "",
    pregens: false,
  });
  const [busy, setBusy] = useState(false);
  const set = <K extends keyof typeof form>(k: K, v: (typeof form)[K]) => setForm((f) => ({ ...f, [k]: v }));

  async function start() {
    setBusy(true);
    try {
      const res = await fetch(`/api/campaigns/${campaignId}/generate`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ ...form, players: Number(form.players) || 4 }),
      });
      const json = await res.json();
      if (!res.ok) {
        const detail = json.error?.details?.[0]?.message;
        throw new Error(detail ?? json.error?.message ?? "Impossible de lancer la génération");
      }
      onStarted(json.job.id);
      toast.success("Génération lancée");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Erreur inconnue");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Votre idée de campagne</CardTitle>
        <CardDescription>
          Le co-MJ prépare bible, menaces, personnages, scènes, indices et cartes. Rien n’est installé dans la
          campagne avant votre relecture.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-1">
          <Label htmlFor="gen-pitch">Idée</Label>
          <Textarea
            id="gen-pitch"
            rows={3}
            value={form.pitch}
            onChange={(e) => set("pitch", e.target.value)}
            placeholder="ex. Un village de pêcheurs dont les morts reviennent chaque marée haute ; une secte vend des « retours » aux familles."
          />
        </div>
        <div className="grid gap-3 md:grid-cols-2">
          <div className="space-y-1">
            <Label htmlFor="gen-tone">Ton</Label>
            <Input id="gen-tone" value={form.tone} onChange={(e) => set("tone", e.target.value)} placeholder="ex. horreur folk, mélancolique" />
          </div>
          <div className="space-y-1">
            <Label htmlFor="gen-themes">Thèmes</Label>
            <Input id="gen-themes" value={form.themes} onChange={(e) => set("themes", e.target.value)} placeholder="ex. deuil, cupidité, rédemption" />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <Label htmlFor="gen-players">Joueurs</Label>
              <Input id="gen-players" type="number" min={1} max={10} value={form.players} onChange={(e) => set("players", e.target.value)} />
            </div>
            <div className="space-y-1">
              <Label htmlFor="gen-level">Niveau</Label>
              <Input id="gen-level" value={form.level} onChange={(e) => set("level", e.target.value)} />
            </div>
          </div>
          <div className="space-y-1">
            <Label htmlFor="gen-length">Format</Label>
            <Select value={form.length} onValueChange={(v) => set("length", v as GenerationInput["length"])}>
              <SelectTrigger id="gen-length">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {Object.entries(LENGTH_LABELS).map(([id, label]) => (
                  <SelectItem key={id} value={id}>
                    {label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
        <div className="space-y-1">
          <Label htmlFor="gen-constraints">Contraintes (facultatif)</Label>
          <Textarea
            id="gen-constraints"
            rows={2}
            value={form.constraints}
            onChange={(e) => set("constraints", e.target.value)}
            placeholder="ex. pas d’araignées, un dragon à la fin, sujets à éviter…"
          />
        </div>
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={form.pregens} onChange={(e) => set("pregens", e.target.checked)} />
          Générer aussi des personnages prêts à jouer
        </label>
        <div className="flex flex-wrap items-center justify-between gap-3 border-t pt-3">
          <p className="text-xs text-muted-foreground">
            Modèle : <span className="font-mono">{status.model}</span> · dépensé {usd(status.spentUsd)}
            {status.budgetUsd !== null ? ` sur ${usd(status.budgetUsd)}` : " (pas de budget défini)"} ·{" "}
            <Link className="underline" href={`/campaigns/${campaignId}/settings`}>
              régler
            </Link>
          </p>
          <Button onClick={start} disabled={disabled || busy || form.pitch.trim().length < 10}>
            <Sparkles className="size-4" /> {busy ? "Lancement…" : "Générer la campagne"}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function JobPanel({
  job,
  hasStory,
  onChanged,
}: {
  job: GenerationJobView;
  hasStory: boolean;
  onChanged: () => void;
}) {
  const router = useRouter();
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const draft = job.result?.draft;
  const nameByRef = new Map((draft?.entities ?? []).map((e) => [e.ref, e.name]));
  const errors = job.result?.issues.filter((i) => i.severity === "error").length ?? 0;

  async function apply() {
    setBusy(true);
    try {
      const res = await fetch(`/api/generation-jobs/${job.id}/apply`, { method: "POST" });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error?.details?.[0]?.message ?? json.error?.message ?? "Application impossible");
      toast.success(`Campagne installée : ${json.entities} fiche(s) créée(s)`);
      onChanged();
      router.push(`/campaigns/${job.campaignId}/story`);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Erreur inconnue");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="space-y-4" data-testid="generation-job" data-status={job.status}>
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2 flex-wrap">
            <CardTitle className="text-base">Génération</CardTitle>
            <Badge variant={job.status === "failed" ? "destructive" : job.status === "running" ? "secondary" : "default"}>
              {STATUS_LABELS[job.status]}
            </Badge>
            <span className="text-xs text-muted-foreground">
              {job.model} · {usd(job.usage.costUsd)} · {job.usage.promptTokens + job.usage.completionTokens} jetons
            </span>
          </div>
        </CardHeader>
        <CardContent className="space-y-2 text-sm">
          <ol className="space-y-1">
            {job.steps.map((s) => (
              <li key={s.id} className="flex items-center gap-2" data-testid={`step-${s.id}`} data-status={s.status}>
                {s.status === "done" ? (
                  <CheckCircle2 className="size-4 text-emerald-500" />
                ) : s.status === "running" ? (
                  <Loader2 className="size-4 animate-spin" />
                ) : s.status === "failed" ? (
                  <CircleX className="size-4 text-destructive" />
                ) : (
                  <CircleDashed className="size-4 text-muted-foreground" />
                )}
                <span>{s.label}</span>
                {s.detail ? <span className="text-muted-foreground">— {s.detail}</span> : null}
              </li>
            ))}
          </ol>
          {job.error ? <p className="text-destructive">{job.error}</p> : null}
        </CardContent>
      </Card>

      {draft && (job.status === "succeeded" || job.status === "applied") ? (
        <>
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Fiches proposées ({draft.entities.length})</CardTitle>
            </CardHeader>
            <CardContent className="grid gap-2 md:grid-cols-2 text-sm">
              {draft.entities.map((e) => (
                <div key={e.ref}>
                  <span className="font-medium">{e.name}</span>{" "}
                  <Badge variant="outline" className="text-[10px]">{ENTITY_TYPE_LABELS[e.type]}</Badge>
                  <p className="text-xs text-muted-foreground line-clamp-2">{e.description}</p>
                </div>
              ))}
            </CardContent>
          </Card>

          <StoryDisplay
            story={draft.story}
            issues={job.result!.issues}
            entityName={(ref) => nameByRef.get(ref) ?? ref}
            yamlEditor={
              <StoryYamlEditor
                story={draft.story}
                saveUrl={`/api/generation-jobs/${job.id}`}
                onSaved={(json) => queryClient.setQueryData(["generation-job", job.id], json)}
              />
            }
          />

          {job.status === "succeeded" ? (
            <div className="flex justify-end gap-2">
              {hasStory ? (
                <AlertDialog>
                  <AlertDialogTrigger asChild>
                    <Button disabled={busy || errors > 0}>Appliquer à la campagne</Button>
                  </AlertDialogTrigger>
                  <AlertDialogContent>
                    <AlertDialogHeader>
                      <AlertDialogTitle>Remplacer le scénario actuel ?</AlertDialogTitle>
                      <AlertDialogDescription>
                        Le scénario de la campagne sera remplacé et l’état du monde remis à zéro. Les fiches proposées
                        s’ajoutent aux fiches existantes, qui ne sont pas supprimées.
                      </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                      <AlertDialogCancel>Annuler</AlertDialogCancel>
                      <AlertDialogAction onClick={apply}>Remplacer</AlertDialogAction>
                    </AlertDialogFooter>
                  </AlertDialogContent>
                </AlertDialog>
              ) : (
                <Button disabled={busy || errors > 0} onClick={apply}>
                  Appliquer à la campagne
                </Button>
              )}
            </div>
          ) : null}
          {errors > 0 ? (
            <p className="text-sm text-destructive text-right">
              Corrigez les {errors} erreur(s) dans l’onglet « Modifier (YAML) » avant d’appliquer.
            </p>
          ) : null}
        </>
      ) : null}
    </div>
  );
}
