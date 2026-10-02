"use client";

import { useRef, useState } from "react";
import { Minus, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cellCenter, cellKey, gridPixelSize, hexPoints, type Cell } from "@/lib/engine/grid";
import type { GameMap, MapCell, MapToken } from "@/lib/engine/story";
import { cn } from "@/lib/utils";

export type TokenKind = "pc" | "enemy" | "npc" | "party";

export interface MapViewProps {
  map: GameMap;
  tokens: MapToken[];
  /** Cases révélées ; absent = pas de brouillard. */
  revealed?: Set<string>;
  /** gm : brouillard translucide ; player : opaque. */
  fog?: "gm" | "player";
  /** Coût par case atteignable (surbrillance de déplacement). */
  reachable?: Map<string, number>;
  selectedToken?: string | null;
  selectedCell?: Cell | null;
  /** Case de la scène en cours. */
  currentCell?: Cell | null;
  tokenInfo: (entityId: string) => { label: string; kind: TokenKind; title?: string };
  onCellClick?: (cell: Cell, mods: { shiftKey: boolean }) => void;
  onCellHover?: (cell: Cell | null) => void;
  /** Peinture au glisser (brouillard) : reçoit les cases traversées au relâchement. */
  onPaint?: (cells: Cell[], firstCellRevealed: boolean) => void;
  className?: string;
}

const TERRAIN_COLORS: [RegExp, string][] = [
  [/for[êe]t|bois|jungle/i, "#2f5d34"],
  [/montagne|colline|falaise|roche/i, "#6b5d4f"],
  [/eau|rivi[èe]re|lac|mer|oc[ée]an|gu[ée]|marais/i, "#2c5a7a"],
  [/village|ville|cit[ée]|bourg|port/i, "#8a6a3b"],
  [/ruine|cimeti[èe]re/i, "#5a5560"],
  [/mur|palissade|roc/i, "#1f1b18"],
  [/feu|lave/i, "#b8461f"],
  [/tr[ôo]ne|autel/i, "#5b3f78"],
  [/route|chemin|pont/i, "#9a8a6a"],
  [/d[ée]sert|sable|plage/i, "#b39a5e"],
  [/neige|glace/i, "#c9d6df"],
  [/plaine|prairie|champ|herbe/i, "#5b7a3a"],
];

function terrainColor(terrain?: string): string | null {
  if (!terrain) return null;
  return TERRAIN_COLORS.find(([re]) => re.test(terrain))?.[1] ?? "#4a4a4a";
}

const TOKEN_COLORS: Record<TokenKind, string> = {
  pc: "#2563eb",
  enemy: "#dc2626",
  npc: "#d97706",
  party: "#16a34a",
};

export function MapView({
  map,
  tokens,
  revealed,
  fog = "gm",
  reachable,
  selectedToken,
  selectedCell,
  currentCell,
  tokenInfo,
  onCellClick,
  onCellHover,
  onPaint,
  className,
}: MapViewProps) {
  const { type, cols, rows } = map.grid;
  const size = type === "hex" ? 22 : 30;
  const { width, height } = gridPixelSize(type, cols, rows, size);
  const [zoom, setZoom] = useState(1);
  const painting = useRef<{ cells: Map<string, Cell>; firstRevealed: boolean } | null>(null);

  const cellsByKey = new Map<string, MapCell>(map.cells.map((c) => [cellKey(c.x, c.y), c]));
  const all: Cell[] = [];
  for (let y = 0; y < rows; y++) for (let x = 0; x < cols; x++) all.push({ x, y });

  const shape = (x: number, y: number, props: React.SVGProps<SVGPolygonElement> & React.SVGProps<SVGRectElement>) => {
    const { cx, cy } = cellCenter(type, x, y, size);
    return type === "hex" ? (
      <polygon points={hexPoints(cx, cy, size)} {...props} />
    ) : (
      <rect x={cx - size / 2} y={cy - size / 2} width={size} height={size} {...props} />
    );
  };

  function endPaint() {
    const p = painting.current;
    painting.current = null;
    if (p && onPaint && p.cells.size) onPaint([...p.cells.values()], p.firstRevealed);
  }

  return (
    <div className={cn("relative", className)}>
      <div className="absolute right-2 top-2 z-10 flex gap-1">
        <Button size="icon-xs" variant="secondary" aria-label="Dézoomer" onClick={() => setZoom((z) => Math.max(1, z / 1.25))}>
          <Minus />
        </Button>
        <Button size="icon-xs" variant="secondary" aria-label="Zoomer" onClick={() => setZoom((z) => Math.min(4, z * 1.25))}>
          <Plus />
        </Button>
      </div>
      <div className="overflow-auto rounded border bg-[#1b1a18] max-h-[70vh]" onPointerUp={endPaint} onPointerLeave={endPaint}>
        <svg
          // Ajusté à la largeur disponible ; le zoom agrandit au-delà (défilement).
          style={{ width: `${zoom * 100}%`, height: "auto" }}
          viewBox={`-2 -2 ${width + 4} ${height + 4}`}
          role="img"
          aria-label={`Carte ${map.name}`}
          data-testid="map-svg"
          className="select-none touch-none"
          onPointerLeave={() => onCellHover?.(null)}
        >
          <defs>
            <pattern id={`hatch-${map.id}`} width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
              <line x1="0" y1="0" x2="0" y2="6" stroke="#000" strokeOpacity="0.55" strokeWidth="3" />
            </pattern>
          </defs>
          {map.backgroundUrl ? (
            <image href={map.backgroundUrl} x={0} y={0} width={width} height={height} preserveAspectRatio="none" opacity={0.85} />
          ) : null}

          {/* Terrain et grille */}
          {all.map(({ x, y }) => {
            const key = cellKey(x, y);
            const c = cellsByKey.get(key);
            const fill = terrainColor(c?.terrain) ?? (map.backgroundUrl ? "transparent" : type === "hex" ? "#4d5b3c" : "#5b554d");
            const reach = reachable?.get(key);
            return (
              <g
                key={key}
                data-cell={key}
                onPointerDown={(e) => {
                  if (onPaint && e.button === 0) {
                    const first = revealed?.has(key) ?? false;
                    painting.current = { cells: new Map([[key, { x, y }]]), firstRevealed: first };
                  }
                }}
                onPointerEnter={(e) => {
                  onCellHover?.({ x, y });
                  if (painting.current && e.buttons === 1) painting.current.cells.set(key, { x, y });
                }}
                onClick={(e) => onCellClick?.({ x, y }, { shiftKey: e.shiftKey })}
                className={onCellClick || onPaint ? "cursor-pointer" : undefined}
              >
                {shape(x, y, { fill, fillOpacity: map.backgroundUrl && fill !== "transparent" ? 0.45 : 1, stroke: "#000", strokeOpacity: 0.35, strokeWidth: 0.8 })}
                {c?.blocked ? shape(x, y, { fill: `url(#hatch-${map.id})`, pointerEvents: "none" }) : null}
                {reach !== undefined ? shape(x, y, { fill: "#38bdf8", fillOpacity: 0.28, pointerEvents: "none" }) : null}
              </g>
            );
          })}

          {/* Libellés, sous-cartes, scènes */}
          {map.cells
            .filter((c) => c.label || c.childMapId || c.sceneId)
            .map((c) => {
              const { cx, cy } = cellCenter(type, c.x, c.y, size);
              return (
                <g key={`l-${c.x},${c.y}`} pointerEvents="none">
                  {c.childMapId ? (
                    <circle cx={cx} cy={cy} r={size * 0.28} fill="none" stroke="#fbbf24" strokeWidth={1.5} strokeDasharray="3 2" />
                  ) : null}
                  {c.label ? (
                    <text
                      x={cx}
                      y={cy + size * 0.95}
                      textAnchor="middle"
                      fontSize={type === "hex" ? 8 : 7}
                      fill="#f5f0e6"
                      stroke="#000"
                      strokeWidth={2.2}
                      paintOrder="stroke"
                    >
                      {c.label}
                    </text>
                  ) : null}
                </g>
              );
            })}

          {/* Case de la scène en cours, case sélectionnée */}
          {currentCell ? shape(currentCell.x, currentCell.y, { fill: "none", stroke: "#fbbf24", strokeWidth: 2.5, pointerEvents: "none" }) : null}
          {selectedCell ? shape(selectedCell.x, selectedCell.y, { fill: "none", stroke: "#fff", strokeWidth: 2, strokeDasharray: "4 2", pointerEvents: "none" }) : null}

          {/* Pions */}
          {tokens.map((t) => {
            const { cx, cy } = cellCenter(type, t.x, t.y, size);
            const info = tokenInfo(t.entityId);
            const selected = selectedToken === t.entityId;
            return (
              <g key={t.entityId} pointerEvents="none" data-token={t.entityId}>
                <title>{info.title ?? info.label}</title>
                <circle cx={cx} cy={cy} r={size * 0.4} fill={TOKEN_COLORS[info.kind]} stroke={selected ? "#fff" : "#000"} strokeWidth={selected ? 3 : 1.5} />
                <text x={cx} y={cy + 3.5} textAnchor="middle" fontSize={10} fontWeight={700} fill="#fff">
                  {info.label}
                </text>
              </g>
            );
          })}

          {/* Brouillard */}
          {revealed
            ? all
                .filter(({ x, y }) => !revealed.has(cellKey(x, y)))
                .map(({ x, y }) => (
                  <g key={`f-${x},${y}`} pointerEvents="none">
                    {shape(x, y, { fill: "#000", fillOpacity: fog === "player" ? 1 : 0.5, stroke: "none" })}
                  </g>
                ))
            : null}
        </svg>
      </div>
    </div>
  );
}

export function MapLegend() {
  return (
    <div className="flex flex-wrap gap-3 text-xs text-muted-foreground">
      <Legend color={TOKEN_COLORS.pc} label="Personnage" />
      <Legend color={TOKEN_COLORS.enemy} label="Adversaire" />
      <Legend color={TOKEN_COLORS.npc} label="PNJ" />
      <Legend color={TOKEN_COLORS.party} label="Groupe" />
      <span className="flex items-center gap-1">
        <span className="inline-block size-3 rounded-full border-2 border-dashed border-amber-400" /> ouvre une carte
      </span>
      <span className="flex items-center gap-1">
        <span className="inline-block size-3 bg-[repeating-linear-gradient(45deg,#000_0,#000_2px,transparent_2px,transparent_4px)] border" />{" "}
        infranchissable
      </span>
    </div>
  );
}

function Legend({ color, label }: { color: string; label: string }) {
  return (
    <span className="flex items-center gap-1">
      <span className="inline-block size-3 rounded-full" style={{ background: color }} /> {label}
    </span>
  );
}
