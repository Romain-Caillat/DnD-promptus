"use client";

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Eye, EyeOff, Map as MapIcon, Move, Paintbrush } from "lucide-react";
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
import { MapLegend, MapView, type TokenKind } from "@/components/maps/map-view";
import { useRuleset } from "@/components/providers/ruleset-provider";
import { cellKey, distance, reachableCells, speedInCells, type Cell } from "@/lib/engine/grid";
import { MAP_LEVEL_LABELS, type CampaignStory, type GameMap } from "@/lib/engine/story";
import type { MapOp } from "@/lib/engine/map-ops";
import {
  PARTY_TOKEN,
  defaultMapId,
  mapTokens,
  revealedCells,
  type WorldState,
} from "@/lib/engine/world";
import type { EntityRow } from "@/lib/db/schema";
import type { ParticipantView } from "@/lib/stores/session-store";
import { cn } from "@/lib/utils";

type Tool = "tokens" | "fog";

export function MapPanel({
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
  const [pickedMapId, setPickedMapId] = useState<string | null>(null);
  const [tool, setTool] = useState<Tool>("tokens");
  const [selectedToken, setSelectedToken] = useState<string | null>(null);
  const [placing, setPlacing] = useState<string>("");
  const [selectedCell, setSelectedCell] = useState<Cell | null>(null);
  const [hover, setHover] = useState<Cell | null>(null);
  const [playerView, setPlayerView] = useState(false);
  const [sceneSeen, setSceneSeen] = useState<string | undefined>(undefined);

  const storyKey = ["session-story", sessionId];
  const { data } = useQuery({
    queryKey: storyKey,
    queryFn: async () => {
      const res = await fetch(`/api/sessions/${sessionId}/story`);
      if (!res.ok) throw new Error("Impossible de charger le scénario");
      return (await res.json()) as { story: CampaignStory; world: WorldState };
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

  if (!data || data.story.maps.length === 0) return null;
  const { story, world } = data;
  // Quand le MJ change de scène, la carte revient à celle de la scène.
  if (world.currentSceneId !== sceneSeen) {
    setSceneSeen(world.currentSceneId);
    setPickedMapId(null);
    setSelectedToken(null);
    setSelectedCell(null);
  }
  const entities = new Map((entData?.entities ?? []).map((e) => [e.id, e]));
  const mapId = pickedMapId ?? defaultMapId(story, world);
  const map = story.maps.find((m) => m.id === mapId) ?? story.maps[0];
  const tokens = mapTokens(map, world);
  const revealed = revealedCells(map, world);
  const scene = story.scenes.find((s) => s.id === world.currentSceneId);
  const currentCell =
    scene?.mapPlacement?.mapId === map.id ? { x: scene.mapPlacement.x, y: scene.mapPlacement.y } : null;
  const shownToPlayers = defaultMapId(story, world) === map.id;

  function tokenInfo(entityId: string): { label: string; kind: TokenKind; title: string } {
    if (entityId === PARTY_TOKEN) return { label: "G", kind: "party", title: "Le groupe" };
    const e = entities.get(entityId);
    const name = e?.name ?? entityId;
    const kind: TokenKind = e?.type === "character" ? "pc" : e?.type === "npc" ? "npc" : "enemy";
    return { label: name.slice(0, 2).toUpperCase(), kind, title: name };
  }

  // Portée de déplacement du pion sélectionné, selon les règles et le niveau de carte.
  function moveBudget(entityId: string): number {
    if (map.level === "campaign") {
      const pace = ruleset.movement.campaign.paces.find((p) => p.id === "normal") ?? ruleset.movement.campaign.paces[0];
      return pace?.cellsPerDay ?? 4;
    }
    if (map.level === "region") return ruleset.movement.region.cellsPerHour;
    const attrs = (entities.get(entityId)?.attributes ?? {}) as Record<string, unknown>;
    return speedInCells(attrs, ruleset.movement.local.cellMeters, ruleset.movement.local.defaultSpeedCells);
  }
  const selected = tokens.find((t) => t.entityId === selectedToken);
  const occupied = new Set(tokens.filter((t) => t.entityId !== selectedToken).map((t) => cellKey(t.x, t.y)));
  const reachable =
    tool === "tokens" && selected
      ? reachableCells(map, selected, moveBudget(selected.entityId), ruleset.movement.local.diagonal, occupied)
      : undefined;

  async function send(op: MapOp, ok?: string) {
    // Mise à jour immédiate de l'affichage pour le brouillard (peinture fluide).
    try {
      const res = await fetch(`/api/sessions/${sessionId}/map`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(op),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error?.message ?? "Action impossible");
      queryClient.setQueryData(storyKey, { story, world: json.world as WorldState });
      if (op.op === "move_token") void queryClient.invalidateQueries({ queryKey: ["timeline", sessionId] });
      if (ok) toast.success(ok);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Action impossible");
    }
  }

  function onCellClick(cell: Cell, mods: { shiftKey: boolean }) {
    if (tool !== "tokens") return;
    const key = cellKey(cell.x, cell.y);
    if (placing) {
      void send({ op: "move_token", mapId: map.id, entityId: placing, x: cell.x, y: cell.y });
      setPlacing("");
      return;
    }
    const onCell = tokens.find((t) => t.x === cell.x && t.y === cell.y);
    if (onCell) {
      setSelectedToken(onCell.entityId === selectedToken ? null : onCell.entityId);
      setSelectedCell(null);
      return;
    }
    if (selected) {
      // Le MJ peut toujours forcer un déplacement hors de portée (Maj+clic).
      if (!reachable?.has(key) && !mods.shiftKey) {
        toast.warning("Hors de portée de déplacement — Maj+clic pour forcer");
        return;
      }
      void send({ op: "move_token", mapId: map.id, entityId: selected.entityId, x: cell.x, y: cell.y });
      setSelectedToken(null);
      return;
    }
    setSelectedCell(selectedCell && selectedCell.x === cell.x && selectedCell.y === cell.y ? null : cell);
  }

  const cellInfo = (c: Cell | null) => (c ? map.cells.find((m) => m.x === c.x && m.y === c.y) : undefined);
  const selectedInfo = cellInfo(selectedCell);
  const hoverInfo = cellInfo(hover);
  const childMap = selectedInfo?.childMapId ? story.maps.find((m) => m.id === selectedInfo.childMapId) : undefined;
  const placeable = [
    ...(map.level === "local" ? [] : [{ id: PARTY_TOKEN, name: "Le groupe" }]),
    ...participants
      .filter((p) => p.entity)
      .map((p) => ({ id: p.state.entityId, name: p.entity!.name })),
  ].filter((p) => !tokens.some((t) => t.entityId === p.id));
  const cellMeters =
    map.level === "campaign" ? ruleset.movement.campaign.cellKm * 1000 : map.level === "region" ? ruleset.movement.region.cellMeters : ruleset.movement.local.cellMeters;
  const fmtDist = (cells: number) => {
    const m = cells * cellMeters;
    return m >= 1000 ? `${(m / 1000).toLocaleString("fr-FR")} km` : `${m.toLocaleString("fr-FR")} m`;
  };

  return (
    <Card data-testid="map-panel">
      <CardContent className="pt-4 space-y-3">
        <div className="flex items-center justify-between gap-2 flex-wrap">
          <div className="flex items-center gap-2">
            <MapIcon className="size-4 text-muted-foreground" />
            <h2 className="text-sm font-semibold uppercase tracking-wide">Carte</h2>
            <Badge variant="outline">{MAP_LEVEL_LABELS[map.level]}</Badge>
            {shownToPlayers ? <Badge>Montrée aux joueurs</Badge> : null}
          </div>
          <div className="flex items-center gap-2 flex-wrap">
            <Select value={map.id} onValueChange={(v) => { setPickedMapId(v); setSelectedToken(null); setSelectedCell(null); }}>
              <SelectTrigger className="w-56 h-8 text-xs" aria-label="Choisir une carte">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {story.maps.map((m: GameMap) => (
                  <SelectItem key={m.id} value={m.id}>
                    {m.name} ({MAP_LEVEL_LABELS[m.level]})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {!shownToPlayers ? (
              <Button size="sm" variant="outline" onClick={() => send({ op: "set_active", mapId: map.id }, "Carte montrée aux joueurs")}>
                Montrer aux joueurs
              </Button>
            ) : null}
          </div>
        </div>

        <div className="flex items-center gap-2 flex-wrap text-xs">
          <div className="flex rounded-md border p-0.5">
            <Button size="xs" variant={tool === "tokens" ? "default" : "ghost"} onClick={() => setTool("tokens")}>
              <Move /> Pions
            </Button>
            <Button size="xs" variant={tool === "fog" ? "default" : "ghost"} onClick={() => { setTool("fog"); setSelectedToken(null); }}>
              <Paintbrush /> Brouillard
            </Button>
          </div>
          {tool === "tokens" ? (
            <Select value={placing} onValueChange={setPlacing} disabled={placeable.length === 0}>
              <SelectTrigger className="h-7 w-48 text-xs" aria-label="Placer un pion">
                <SelectValue placeholder={placeable.length ? "Placer un pion…" : "Tous les pions sont placés"} />
              </SelectTrigger>
              <SelectContent>
                {placeable.map((p) => (
                  <SelectItem key={p.id} value={p.id}>
                    {p.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          ) : (
            <>
              <Button size="xs" variant="outline" onClick={() => send({ op: "reveal_all", mapId: map.id })}>
                <Eye /> Tout révéler
              </Button>
              <Button size="xs" variant="outline" onClick={() => send({ op: "hide_all", mapId: map.id })}>
                <EyeOff /> Tout cacher
              </Button>
            </>
          )}
          <label className="flex items-center gap-1 ml-auto">
            <input type="checkbox" checked={playerView} onChange={(e) => setPlayerView(e.target.checked)} /> Vue joueurs
          </label>
        </div>

        <p className="text-xs text-muted-foreground min-h-4" data-testid="map-hint">
          {tool === "fog"
            ? "Cliquez ou glissez sur les cases pour les révéler (ou les cacher si la première case est déjà révélée)."
            : placing
              ? `Cliquez sur une case pour placer ${placeable.find((p) => p.id === placing)?.name ?? "le pion"}.`
              : selected
                ? `${tokenInfo(selected.entityId).title} : portée ${moveBudget(selected.entityId)} case(s) (Maj+clic pour forcer)` +
                  (hover ? ` · case visée à ${distance(map.grid.type, selected, hover, ruleset.movement.local.diagonal)} case(s), ${fmtDist(distance(map.grid.type, selected, hover, ruleset.movement.local.diagonal))}` : "")
                : hover
                  ? `(${hover.x}, ${hover.y})${hoverInfo?.label ? ` ${hoverInfo.label}` : ""}${hoverInfo?.terrain ? ` · ${hoverInfo.terrain}` : ""}${hoverInfo?.blocked ? " · infranchissable" : ""}`
                  : "Cliquez sur un pion pour voir sa portée, puis sur une case pour le déplacer."}
        </p>

        <MapView
          map={map}
          tokens={tokens}
          revealed={revealed}
          fog={playerView ? "player" : "gm"}
          reachable={reachable}
          selectedToken={selectedToken}
          selectedCell={selectedCell}
          currentCell={currentCell}
          tokenInfo={tokenInfo}
          onCellClick={tool === "tokens" ? onCellClick : undefined}
          onCellHover={setHover}
          onPaint={
            tool === "fog"
              ? (cells, firstRevealed) =>
                  send({ op: firstRevealed ? "hide" : "reveal", mapId: map.id, cells: cells.map((c) => [c.x, c.y]) })
              : undefined
          }
        />

        <div className="flex items-center justify-between gap-2 flex-wrap">
          <MapLegend />
          {selectedCell ? (
            <div className={cn("text-xs flex items-center gap-2")}>
              <span>
                ({selectedCell.x}, {selectedCell.y}) {selectedInfo?.label ?? selectedInfo?.terrain ?? ""}
              </span>
              {childMap ? (
                <Button size="xs" onClick={() => { setPickedMapId(childMap.id); setSelectedCell(null); }}>
                  Ouvrir « {childMap.name} »
                </Button>
              ) : null}
            </div>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}
