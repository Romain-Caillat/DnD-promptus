"use client";

import { useState, useSyncExternalStore } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Eye, Footprints, Swords, X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { MapLegend, MapView, type TokenKind } from "@/components/maps/map-view";
import { PHASE_LABELS } from "@/lib/engine/catalog";
import { cellKey, reachableCells, type Cell } from "@/lib/engine/grid";
import type { PlayerView } from "@/lib/play/projection";
import { useRealtime } from "@/lib/realtime/use-realtime";
import { cn } from "@/lib/utils";

// ----------------------------------------------------------------------------
// Jeton du joueur (localStorage, propre à chaque lien d'invitation)
// ----------------------------------------------------------------------------

const listeners = new Set<() => void>();
const storageKey = (code: string) => `promptus:player:${code}`;

function readToken(code: string): string | null {
  try {
    return localStorage.getItem(storageKey(code));
  } catch {
    return null;
  }
}

function writeToken(code: string, token: string | null) {
  try {
    if (token) localStorage.setItem(storageKey(code), token);
    else localStorage.removeItem(storageKey(code));
  } catch {
    // Stockage indisponible : le joueur devra rejoindre à nouveau.
  }
  for (const l of listeners) l();
}

function usePlayerToken(code: string): string | null | undefined {
  return useSyncExternalStore(
    (cb) => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    () => readToken(code),
    () => undefined,
  );
}

async function errorMessage(res: Response, fallback: string): Promise<string> {
  const json = (await res.json().catch(() => null)) as { error?: { message?: string } } | null;
  return json?.error?.message ?? fallback;
}

export function PlayerApp({ code }: { code: string }) {
  const token = usePlayerToken(code);
  if (token === undefined) return null;
  return token ? <PlayerScreen code={code} token={token} /> : <JoinScreen code={code} />;
}

// ----------------------------------------------------------------------------
// Accueil : pseudo + choix du personnage
// ----------------------------------------------------------------------------

interface Lobby {
  sessionName: string;
  campaignName: string;
  characters: { id: string; name: string; description: string | null; imageUrl: string | null; taken: boolean }[];
}

function JoinScreen({ code }: { code: string }) {
  const [name, setName] = useState("");
  const [characterId, setCharacterId] = useState<string | null>(null);
  const lobby = useQuery({
    queryKey: ["play-lobby", code],
    queryFn: async () => {
      const res = await fetch(`/api/play/${code}/lobby`);
      if (!res.ok) throw new Error(await errorMessage(res, "Lien d’invitation invalide"));
      return (await res.json()) as Lobby;
    },
  });
  const join = useMutation({
    mutationFn: async () => {
      const res = await fetch(`/api/play/${code}/join`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ name, characterEntityId: characterId }),
      });
      if (!res.ok) throw new Error(await errorMessage(res, "Impossible de rejoindre"));
      return (await res.json()) as { token: string };
    },
    onSuccess: ({ token }) => writeToken(code, token),
    onError: (e) => {
      toast.error(e.message);
      void lobby.refetch();
    },
  });

  if (lobby.isError) {
    return <p className="p-6 text-center text-destructive">{lobby.error.message}</p>;
  }
  if (!lobby.data) return <p className="p-6 text-center text-muted-foreground">Chargement…</p>;

  return (
    <main className="mx-auto w-full max-w-md p-4 space-y-4">
      <header className="text-center space-y-1">
        <p className="text-xs uppercase tracking-wide text-muted-foreground">{lobby.data.campaignName}</p>
        <h1 className="text-2xl font-semibold">{lobby.data.sessionName}</h1>
      </header>
      <form
        className="space-y-4"
        onSubmit={(e) => {
          e.preventDefault();
          join.mutate();
        }}
      >
        <div className="space-y-1">
          <Label htmlFor="player-name">Votre pseudo</Label>
          <Input id="player-name" value={name} onChange={(e) => setName(e.target.value)} maxLength={40} autoFocus />
        </div>
        <fieldset className="space-y-2">
          <legend className="text-sm font-medium mb-1">Votre personnage</legend>
          {lobby.data.characters.map((c) => (
            <button
              key={c.id}
              type="button"
              disabled={c.taken}
              onClick={() => setCharacterId(c.id)}
              aria-pressed={characterId === c.id}
              className={cn(
                "w-full rounded-lg border p-3 text-left transition-colors disabled:opacity-50",
                characterId === c.id ? "border-primary bg-primary/10" : "hover:bg-muted/50",
              )}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-medium">{c.name}</span>
                {c.taken ? <Badge variant="secondary">pris</Badge> : null}
              </div>
              {c.description ? <p className="mt-1 text-xs text-muted-foreground line-clamp-2">{c.description}</p> : null}
            </button>
          ))}
          <button
            type="button"
            onClick={() => setCharacterId(null)}
            aria-pressed={characterId === null}
            className={cn(
              "w-full rounded-lg border p-3 text-left text-sm",
              characterId === null ? "border-primary bg-primary/10" : "hover:bg-muted/50",
            )}
          >
            Spectateur (sans personnage)
          </button>
        </fieldset>
        <Button type="submit" className="w-full" disabled={!name.trim() || join.isPending}>
          Rejoindre la partie
        </Button>
      </form>
    </main>
  );
}

// ----------------------------------------------------------------------------
// Écran de jeu
// ----------------------------------------------------------------------------

function PlayerScreen({ code, token }: { code: string; token: string }) {
  const queryClient = useQueryClient();
  const key = ["play", code];
  const headers = { "content-type": "application/json", "x-player-token": token };

  const { data: view, error } = useQuery({
    queryKey: key,
    queryFn: async () => {
      const res = await fetch(`/api/play/${code}/state`, { headers });
      if (res.status === 401 || res.status === 404) {
        writeToken(code, null);
        throw new Error(await errorMessage(res, "Session introuvable"));
      }
      if (!res.ok) throw new Error(await errorMessage(res, "Impossible de charger la partie"));
      return (await res.json()) as PlayerView;
    },
  });
  const realtime = useRealtime({ type: "hello", role: "player", inviteCode: code, token }, () => [key]);

  if (error && !view) return <p className="p-6 text-center text-destructive">{error.message}</p>;
  if (!view) return <p className="p-6 text-center text-muted-foreground">Chargement…</p>;

  const refresh = () => void queryClient.invalidateQueries({ queryKey: key });

  return (
    <main className="mx-auto w-full max-w-6xl p-3 sm:p-4 space-y-3" data-testid="player-screen">
      <header className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="text-xs uppercase tracking-wide text-muted-foreground truncate">{view.session.campaignName}</p>
          <h1 className="text-lg font-semibold truncate">{view.character?.name ?? view.me.name}</h1>
        </div>
        <div className="flex items-center gap-2 text-xs">
          <Badge variant="outline" data-testid="player-phase">
            {PHASE_LABELS[view.session.phase]}
            {view.session.phase === "combat" && view.session.combatRound > 0 ? ` · round ${view.session.combatRound}` : ""}
          </Badge>
          <span className={cn("flex items-center gap-1", realtime.connected ? "text-emerald-500" : "text-muted-foreground")}>
            <span className={cn("size-2 rounded-full", realtime.connected ? "bg-emerald-500" : "bg-muted-foreground")} />
            {realtime.connected ? (realtime.gmOnline ? "MJ en ligne" : "MJ absent") : "Hors ligne"}
          </span>
        </div>
      </header>

      {view.isMyTurn ? (
        <div className="rounded-lg border border-amber-500 bg-amber-500/15 p-2 text-center font-semibold text-amber-300" data-testid="my-turn">
          <Swords className="inline size-4 mr-1" /> C’est votre tour !
        </div>
      ) : null}

      <Spotlight spotlight={view.spotlight} />

      <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_380px]">
        <div className="space-y-3 min-w-0">
          {view.scene ? (
            <Card>
              <CardContent className="pt-4 space-y-1">
                <h2 className="font-semibold" data-testid="player-scene">{view.scene.title}</h2>
                {view.scene.readAloud ? <p className="text-sm italic whitespace-pre-line">{view.scene.readAloud}</p> : null}
              </CardContent>
            </Card>
          ) : null}
          <PlayerMapCard code={code} headers={headers} view={view} onMoved={refresh} />
        </div>

        <Tabs defaultValue={view.character ? "actions" : "journal"} className="min-w-0">
          <TabsList className="w-full">
            {view.character ? <TabsTrigger value="actions">Actions</TabsTrigger> : null}
            {view.character ? <TabsTrigger value="sheet">Fiche</TabsTrigger> : null}
            <TabsTrigger value="party">Groupe</TabsTrigger>
            <TabsTrigger value="journal">Journal</TabsTrigger>
          </TabsList>
          {view.character ? (
            <TabsContent value="actions">
              <ActionsPanel code={code} headers={headers} view={view} onSent={refresh} />
            </TabsContent>
          ) : null}
          {view.character ? (
            <TabsContent value="sheet">
              <SheetPanel character={view.character} />
            </TabsContent>
          ) : null}
          <TabsContent value="party">
            <PartyPanel view={view} />
          </TabsContent>
          <TabsContent value="journal">
            <JournalPanel view={view} />
          </TabsContent>
        </Tabs>
      </div>
    </main>
  );
}

function Spotlight({ spotlight }: { spotlight: PlayerView["spotlight"] }) {
  const [dismissed, setDismissed] = useState<string | null>(null);
  if (!spotlight || dismissed === spotlight.name) return null;
  return (
    <Card className="border-primary" data-testid="player-spotlight">
      <CardContent className="pt-4 flex gap-3">
        {spotlight.imageUrl ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img src={spotlight.imageUrl} alt={spotlight.name} className="size-24 rounded object-cover shrink-0" />
        ) : null}
        <div className="min-w-0 flex-1 space-y-1">
          <p className="text-xs uppercase tracking-wide text-muted-foreground flex items-center gap-1">
            <Eye className="size-3" /> Le MJ vous montre
          </p>
          <h2 className="font-semibold">{spotlight.name}</h2>
          {spotlight.description ? <p className="text-sm whitespace-pre-line">{spotlight.description}</p> : null}
        </div>
        <Button size="icon-sm" variant="ghost" aria-label="Fermer" onClick={() => setDismissed(spotlight.name)}>
          <X />
        </Button>
      </CardContent>
    </Card>
  );
}

// ----------------------------------------------------------------------------
// Carte : zone révélée, déplacement de son pion selon les règles
// ----------------------------------------------------------------------------

function PlayerMapCard({
  code,
  headers,
  view,
  onMoved,
}: {
  code: string;
  headers: Record<string, string>;
  view: PlayerView;
  onMoved: () => void;
}) {
  const move = useMutation({
    mutationFn: async (cell: Cell) => {
      const res = await fetch(`/api/play/${code}/move`, { method: "POST", headers, body: JSON.stringify(cell) });
      if (!res.ok) throw new Error(await errorMessage(res, "Déplacement refusé"));
      return (await res.json()) as { cost: number };
    },
    onSuccess: onMoved,
    onError: (e) => toast.error(e.message),
  });

  const map = view.map;
  if (!map) {
    return (
      <Card>
        <CardContent className="pt-4 text-sm text-muted-foreground italic">Aucune carte n’est affichée pour le moment.</CardContent>
      </Card>
    );
  }

  const revealed = new Set(map.revealed);
  const mine = map.tokens.find((t) => t.mine);
  const mv = view.movement;
  let reachable: Map<string, number> | undefined;
  if (mine && mv?.allowed) {
    // Le brouillard et les autres pions bloquent, comme côté serveur.
    const blocked = new Set<string>();
    for (let y = 0; y < map.grid.rows; y++)
      for (let x = 0; x < map.grid.cols; x++) if (!revealed.has(cellKey(x, y))) blocked.add(cellKey(x, y));
    for (const t of map.tokens) if (!t.mine) blocked.add(cellKey(t.x, t.y));
    reachable = reachableCells(map, mine, mv.budgetCells - mv.usedCells, mv.diagonal, blocked);
    reachable.delete(cellKey(mine.x, mine.y));
  }
  const labels = new Map(map.tokens.map((t) => [t.entityId, t]));

  return (
    <Card>
      <CardContent className="pt-4 space-y-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="font-semibold text-sm">{map.name}</h2>
          {mv ? (
            <span className="text-xs text-muted-foreground flex items-center gap-1" data-testid="movement-info">
              <Footprints className="size-3" />
              {mv.reason ?? `${mv.budgetCells - mv.usedCells} / ${mv.budgetCells} case(s) — touchez une case en surbrillance`}
            </span>
          ) : null}
        </div>
        <MapView
          map={{ ...map, tokens: [] }}
          tokens={map.tokens}
          revealed={revealed}
          fog="player"
          reachable={reachable}
          selectedToken={mine?.entityId ?? null}
          tokenInfo={(id) => {
            const t = labels.get(id);
            const label = t?.label ?? "?";
            return { label: label.slice(0, 2), kind: (t?.kind ?? "enemy") as TokenKind, title: label };
          }}
          onCellClick={(cell) => {
            if (!reachable?.has(cellKey(cell.x, cell.y)) || move.isPending) return;
            move.mutate(cell);
          }}
        />
        <MapLegend />
      </CardContent>
    </Card>
  );
}

// ----------------------------------------------------------------------------
// Onglets
// ----------------------------------------------------------------------------

function ActionsPanel({
  code,
  headers,
  view,
  onSent,
}: {
  code: string;
  headers: Record<string, string>;
  view: PlayerView;
  onSent: () => void;
}) {
  const [selected, setSelected] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const send = useMutation({
    mutationFn: async () => {
      const res = await fetch(`/api/play/${code}/requests`, {
        method: "POST",
        headers,
        body: JSON.stringify({ actionId: selected, note: note || undefined }),
      });
      if (!res.ok) throw new Error(await errorMessage(res, "Demande refusée"));
    },
    onSuccess: () => {
      toast.success("Demande envoyée au MJ");
      setSelected(null);
      setNote("");
      onSent();
    },
    onError: (e) => toast.error(e.message),
  });
  const action = view.actions.find((a) => a.id === selected);

  return (
    <div className="space-y-3">
      <p className="text-xs text-muted-foreground">
        Actions possibles en {PHASE_LABELS[view.session.phase].toLowerCase()}. Le MJ valide chaque demande.
      </p>
      <div className="grid grid-cols-2 gap-2">
        {view.actions.map((a) => (
          <button
            key={a.id}
            type="button"
            onClick={() => setSelected(a.id === selected ? null : a.id)}
            aria-pressed={a.id === selected}
            className={cn(
              "rounded-lg border p-2 text-left text-sm",
              a.id === selected ? "border-primary bg-primary/10" : "hover:bg-muted/50",
            )}
          >
            <span className="font-medium">{a.label}</span>
            {a.detail ? <span className="block text-xs text-muted-foreground">{a.detail}</span> : null}
          </button>
        ))}
      </div>
      {action ? (
        <div className="space-y-2 rounded-lg border p-3">
          {action.description ? <p className="text-xs text-muted-foreground">{action.description}</p> : null}
          <Textarea
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="Précisez (cible, intention…) — facultatif"
            maxLength={500}
            rows={2}
          />
          <Button className="w-full" onClick={() => send.mutate()} disabled={send.isPending}>
            Demander : {action.label}
          </Button>
        </div>
      ) : null}
      {view.requests.length ? (
        <div className="space-y-1">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Mes demandes</h3>
          <ul className="space-y-1" data-testid="my-requests">
            {view.requests.map((r) => (
              <li key={r.id} className="rounded border p-2 text-sm">
                <div className="flex items-center justify-between gap-2">
                  <span>{r.label}</span>
                  <Badge variant={r.status === "pending" ? "outline" : r.status === "rejected" ? "destructive" : "secondary"}>
                    {r.status === "pending" ? "en attente" : r.status === "rejected" ? "refusée" : "résolue"}
                  </Badge>
                </div>
                {r.result ? <p className="mt-1 text-xs text-muted-foreground">{r.result}</p> : null}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}

function SheetPanel({ character }: { character: NonNullable<PlayerView["character"]> }) {
  const fmt = (n: number) => (n >= 0 ? `+${n}` : String(n));
  return (
    <div className="space-y-3" data-testid="player-sheet">
      <div className="grid grid-cols-2 gap-2 text-center">
        <Stat label="Points de vie" value={character.hp !== null ? `${character.hp}${character.hpMax !== null ? ` / ${character.hpMax}` : ""}` : "—"} />
        <Stat label="Classe d’armure" value={character.ac !== null ? String(character.ac) : "—"} />
      </div>
      <div className="grid grid-cols-3 gap-2 text-center">
        {character.abilities.map((a) => (
          <div key={a.id} className="rounded border p-2" title={a.label}>
            <div className="text-xs text-muted-foreground">{a.abbr}</div>
            <div className="font-semibold">{fmt(a.modifier)}</div>
            <div className="text-xs text-muted-foreground">{a.score}</div>
          </div>
        ))}
      </div>
      {character.conditions.length ? (
        <div className="flex flex-wrap gap-1">
          {character.conditions.map((c) => (
            <Badge key={c.id} variant="destructive">
              {c.label}
              {c.remainingRounds ? ` (${c.remainingRounds})` : ""}
            </Badge>
          ))}
        </div>
      ) : null}
      {character.resources.length ? (
        <ul className="text-sm space-y-1">
          {character.resources.map((r) => (
            <li key={r.id} className="flex justify-between">
              <span>{r.id}</span>
              <span>
                {r.current} / {r.max}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
      {character.inventory.length ? (
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Inventaire</h3>
          <p className="text-sm">{character.inventory.join(", ")}</p>
        </div>
      ) : null}
      {character.description ? <p className="text-sm text-muted-foreground whitespace-pre-line">{character.description}</p> : null}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded border p-2">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="text-lg font-semibold">{value}</div>
    </div>
  );
}

function PartyPanel({ view }: { view: PlayerView }) {
  return (
    <div className="space-y-3">
      {view.initiative.length ? (
        <div>
          <h3 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-1">Initiative</h3>
          <ol className="space-y-1 text-sm" data-testid="player-initiative">
            {view.initiative.map((e, i) => (
              <li
                key={i}
                className={cn("flex items-center gap-2 rounded px-2 py-1", e.active && "bg-amber-500/15 font-semibold", e.mine && "text-primary")}
              >
                <span className={cn("size-2 rounded-full", e.isPlayer ? "bg-blue-500" : "bg-red-500")} />
                {e.name}
              </li>
            ))}
          </ol>
        </div>
      ) : null}
      <ul className="space-y-1 text-sm">
        {view.party.map((p) => (
          <li key={p.id} className="flex items-center justify-between rounded border px-2 py-1">
            <span>
              {p.name}
              {p.player ? <span className="text-xs text-muted-foreground"> · {p.player}</span> : null}
            </span>
            {p.hp !== null ? (
              <span className="text-xs">
                {p.hp}
                {p.hpMax !== null ? ` / ${p.hpMax}` : ""} PV
              </span>
            ) : null}
          </li>
        ))}
      </ul>
    </div>
  );
}

function JournalPanel({ view }: { view: PlayerView }) {
  if (!view.timeline.length) return <p className="text-sm text-muted-foreground italic">Rien pour l’instant.</p>;
  return (
    <ul className="space-y-1 text-sm max-h-[60vh] overflow-auto" data-testid="player-journal">
      {view.timeline.map((t) => (
        <li key={t.id} className="border-b pb-1">
          {t.description}
        </li>
      ))}
    </ul>
  );
}
