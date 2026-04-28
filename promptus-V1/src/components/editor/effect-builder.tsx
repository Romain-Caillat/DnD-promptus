"use client";

import { useState } from "react";
import { ArrowDown, ArrowUp, Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  DAMAGE_TYPES,
  EFFECT_KINDS,
  RESOURCE_KINDS,
  STANDARD_CONDITIONS,
  STATS,
  type EffectKindId,
} from "@/lib/engine/catalog";
import type { Effect, TargetSpec } from "@/lib/engine/types";
import { TargetField } from "./target-field";

interface Props {
  value: Effect[];
  onChange: (next: Effect[]) => void;
  /** Used to generate stable nested ids for accessibility. */
  idPrefix?: string;
}

const DEFAULT_TARGET: TargetSpec = { type: "self" };

function defaultEffectFor(kind: EffectKindId): Effect {
  switch (kind) {
    case "damage":
      return { type: "damage", amount: "1d6", damageType: "slashing", target: { ...DEFAULT_TARGET } };
    case "heal":
      return { type: "heal", amount: "1d8", target: { ...DEFAULT_TARGET } };
    case "apply_condition":
      return {
        type: "apply_condition",
        conditionId: "poisoned",
        target: { ...DEFAULT_TARGET },
        duration: { rounds: 1 },
      };
    case "remove_condition":
      return { type: "remove_condition", conditionId: "poisoned", target: { ...DEFAULT_TARGET } };
    case "modify_stat":
      return { type: "modify_stat", stat: "STR", modifier: 1, target: { ...DEFAULT_TARGET }, duration: { rounds: 1 } };
    case "roll_check":
      return { type: "roll_check", stat: "DEX", dc: 10, target: { ...DEFAULT_TARGET }, outcomeSuccess: [], outcomeFail: [] };
    case "consume_resource":
      return { type: "consume_resource", resource: "spell_slot", amount: 1, target: { type: "caster" } };
    case "restore_resource":
      return { type: "restore_resource", resource: "spell_slot", amount: 1, target: { type: "caster" } };
    case "set_state":
      return { type: "set_state", entityId: "", attribute: "status", value: "alive" };
    case "move_entity":
      return { type: "move_entity", entityId: "", toLocationId: "" };
    case "reveal_entity":
      return { type: "reveal_entity", entityId: "", toUsers: "all_players" };
    case "set_relation":
      return { type: "set_relation", fromId: "", toId: "", delta: 0 };
    case "trigger_event":
      return { type: "trigger_event", eventId: "" };
    case "add_to_inventory":
      return { type: "add_to_inventory", itemId: "", target: { ...DEFAULT_TARGET }, quantity: 1 };
    case "remove_from_inventory":
      return { type: "remove_from_inventory", itemId: "", target: { ...DEFAULT_TARGET }, quantity: 1 };
    case "play_ambience":
      return { type: "play_ambience", ambienceId: "" };
    case "play_music":
      return { type: "play_music", musicId: "" };
    case "play_sound":
      return { type: "play_sound", soundId: "" };
    case "display_image":
      return { type: "display_image", prompt: "" };
    case "display_text":
      return { type: "display_text", text: "" };
    default:
      // Shouldn't happen; fallback
      return { type: "display_text", text: "" } as Effect;
  }
}

export function EffectBuilder({ value, onChange, idPrefix = "fx" }: Props) {
  const [pendingKind, setPendingKind] = useState<EffectKindId | "">("");

  function addEffect() {
    if (!pendingKind) return;
    onChange([...value, defaultEffectFor(pendingKind)]);
    setPendingKind("");
  }

  function updateAt(index: number, next: Effect) {
    onChange(value.map((e, i) => (i === index ? next : e)));
  }

  function removeAt(index: number) {
    onChange(value.filter((_, i) => i !== index));
  }

  function moveUp(index: number) {
    if (index === 0) return;
    const next = [...value];
    [next[index - 1], next[index]] = [next[index], next[index - 1]];
    onChange(next);
  }

  function moveDown(index: number) {
    if (index === value.length - 1) return;
    const next = [...value];
    [next[index + 1], next[index]] = [next[index], next[index + 1]];
    onChange(next);
  }

  return (
    <div className="space-y-4">
      <div className="flex items-end gap-2">
        <div className="flex-1">
          <Label htmlFor={`${idPrefix}-add`} className="text-xs uppercase tracking-wide text-muted-foreground">
            Add an effect
          </Label>
          <Select value={pendingKind} onValueChange={(v) => setPendingKind(v as EffectKindId)}>
            <SelectTrigger id={`${idPrefix}-add`}>
              <SelectValue placeholder="Pick an effect kind…" />
            </SelectTrigger>
            <SelectContent>
              {(["combat", "narration", "sensory"] as const).map((cat) => (
                <div key={cat}>
                  <div className="px-2 py-1 text-xs uppercase text-muted-foreground capitalize">{cat}</div>
                  {EFFECT_KINDS.filter((k) => k.category === cat).map((k) => (
                    <SelectItem key={k.id} value={k.id}>
                      {k.label}
                    </SelectItem>
                  ))}
                </div>
              ))}
            </SelectContent>
          </Select>
        </div>
        <Button type="button" onClick={addEffect} disabled={!pendingKind}>
          <Plus className="size-4" /> Add
        </Button>
      </div>

      {value.length === 0 ? (
        <p className="text-sm text-muted-foreground italic">
          No effects yet. Add one above to make this entity do something.
        </p>
      ) : (
        <div className="space-y-3">
          {value.map((effect, index) => (
            <Card key={index}>
              <CardContent className="space-y-3 pt-4">
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    <Badge variant="secondary" className="text-xs">
                      #{index + 1}
                    </Badge>
                    <span className="text-sm font-medium capitalize">
                      {effect.type.replace(/_/g, " ")}
                    </span>
                  </div>
                  <div className="flex gap-1">
                    <Button type="button" variant="ghost" size="icon" onClick={() => moveUp(index)} disabled={index === 0} aria-label="Move up">
                      <ArrowUp className="size-4" />
                    </Button>
                    <Button type="button" variant="ghost" size="icon" onClick={() => moveDown(index)} disabled={index === value.length - 1} aria-label="Move down">
                      <ArrowDown className="size-4" />
                    </Button>
                    <Button type="button" variant="ghost" size="icon" onClick={() => removeAt(index)} aria-label="Delete effect">
                      <Trash2 className="size-4 text-destructive" />
                    </Button>
                  </div>
                </div>
                <Separator />
                <EffectFields
                  effect={effect}
                  onChange={(next) => updateAt(index, next)}
                  idPrefix={`${idPrefix}-${index}`}
                />
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

// ----------------------------------------------------------------------------
// Per-effect field set
// ----------------------------------------------------------------------------

function EffectFields({
  effect,
  onChange,
  idPrefix,
}: {
  effect: Effect;
  onChange: (next: Effect) => void;
  idPrefix: string;
}) {
  switch (effect.type) {
    case "damage":
      return (
        <div className="grid gap-3 md:grid-cols-2">
          <Field label="Amount (dice notation)" id={`${idPrefix}-amount`}>
            <Input
              id={`${idPrefix}-amount`}
              value={effect.amount}
              onChange={(e) => onChange({ ...effect, amount: e.target.value })}
              placeholder="1d8+3"
            />
          </Field>
          <Field label="Damage type" id={`${idPrefix}-dtype`}>
            <Select value={effect.damageType} onValueChange={(v) => onChange({ ...effect, damageType: v as Effect["type"] extends "damage" ? typeof effect.damageType : never })}>
              <SelectTrigger id={`${idPrefix}-dtype`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {DAMAGE_TYPES.map((d) => (
                  <SelectItem key={d} value={d}>{d}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <div className="md:col-span-2">
            <TargetField value={effect.target} onChange={(t) => onChange({ ...effect, target: t })} idPrefix={idPrefix} />
          </div>
        </div>
      );

    case "heal":
      return (
        <div className="grid gap-3 md:grid-cols-2">
          <Field label="Amount" id={`${idPrefix}-amount`}>
            <Input id={`${idPrefix}-amount`} value={effect.amount} onChange={(e) => onChange({ ...effect, amount: e.target.value })} placeholder="2d4+2" />
          </Field>
          <div className="md:col-span-2">
            <TargetField value={effect.target} onChange={(t) => onChange({ ...effect, target: t })} idPrefix={idPrefix} />
          </div>
        </div>
      );

    case "apply_condition":
      return (
        <div className="grid gap-3 md:grid-cols-2">
          <Field label="Condition" id={`${idPrefix}-cond`}>
            <Select value={effect.conditionId} onValueChange={(v) => onChange({ ...effect, conditionId: v })}>
              <SelectTrigger id={`${idPrefix}-cond`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {STANDARD_CONDITIONS.map((c) => (
                  <SelectItem key={c} value={c}>{c}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <Field label="Duration (rounds)" id={`${idPrefix}-dur`}>
            <Input
              id={`${idPrefix}-dur`}
              type="number"
              min={0}
              value={effect.duration?.rounds ?? ""}
              onChange={(e) => onChange({ ...effect, duration: { rounds: e.target.value ? Number(e.target.value) : undefined } })}
              placeholder="1"
            />
          </Field>
          <div className="md:col-span-2">
            <TargetField value={effect.target} onChange={(t) => onChange({ ...effect, target: t })} idPrefix={idPrefix} />
          </div>
        </div>
      );

    case "remove_condition":
      return (
        <div className="grid gap-3 md:grid-cols-2">
          <Field label="Condition" id={`${idPrefix}-cond`}>
            <Select value={effect.conditionId} onValueChange={(v) => onChange({ ...effect, conditionId: v })}>
              <SelectTrigger id={`${idPrefix}-cond`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {STANDARD_CONDITIONS.map((c) => (
                  <SelectItem key={c} value={c}>{c}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <div className="md:col-span-2">
            <TargetField value={effect.target} onChange={(t) => onChange({ ...effect, target: t })} idPrefix={idPrefix} />
          </div>
        </div>
      );

    case "modify_stat":
      return (
        <div className="grid gap-3 md:grid-cols-3">
          <Field label="Stat" id={`${idPrefix}-stat`}>
            <Input id={`${idPrefix}-stat`} value={effect.stat} onChange={(e) => onChange({ ...effect, stat: e.target.value })} placeholder="STR | AC | speed" />
          </Field>
          <Field label="Modifier" id={`${idPrefix}-mod`}>
            <Input id={`${idPrefix}-mod`} type="number" value={effect.modifier} onChange={(e) => onChange({ ...effect, modifier: Number(e.target.value) })} />
          </Field>
          <Field label="Duration (rounds)" id={`${idPrefix}-dur`}>
            <Input
              id={`${idPrefix}-dur`}
              type="number"
              min={0}
              value={effect.duration?.rounds ?? ""}
              onChange={(e) => onChange({ ...effect, duration: { rounds: e.target.value ? Number(e.target.value) : undefined } })}
            />
          </Field>
          <div className="md:col-span-3">
            <TargetField value={effect.target} onChange={(t) => onChange({ ...effect, target: t })} idPrefix={idPrefix} />
          </div>
        </div>
      );

    case "roll_check":
      return (
        <div className="space-y-3">
          <div className="grid gap-3 md:grid-cols-2">
            <Field label="Saving throw / check" id={`${idPrefix}-stat`}>
              <Select value={effect.stat} onValueChange={(v) => onChange({ ...effect, stat: v as typeof effect.stat })}>
                <SelectTrigger id={`${idPrefix}-stat`}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {STATS.map((s) => (
                    <SelectItem key={s} value={s}>{s}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
            <Field label="DC (number or expression)" id={`${idPrefix}-dc`}>
              <Input
                id={`${idPrefix}-dc`}
                value={String(effect.dc)}
                onChange={(e) => {
                  const num = Number(e.target.value);
                  onChange({ ...effect, dc: Number.isNaN(num) ? e.target.value : num });
                }}
                placeholder="14"
              />
            </Field>
          </div>
          <TargetField value={effect.target} onChange={(t) => onChange({ ...effect, target: t })} idPrefix={idPrefix} />
          <div className="rounded border bg-muted/30 p-3 space-y-2">
            <p className="text-xs uppercase tracking-wide text-muted-foreground">
              On success / on failure: nested effects are configured separately
              (V1 enrichment). For MVP, leave them empty and resolve outcomes manually,
              or use damage with “/2” notation.
            </p>
          </div>
        </div>
      );

    case "consume_resource":
    case "restore_resource":
      return (
        <div className="grid gap-3 md:grid-cols-3">
          <Field label="Resource" id={`${idPrefix}-res`}>
            <Select value={effect.resource} onValueChange={(v) => onChange({ ...effect, resource: v as typeof effect.resource })}>
              <SelectTrigger id={`${idPrefix}-res`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {RESOURCE_KINDS.map((r) => (
                  <SelectItem key={r} value={r}>{r}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <Field label="Amount" id={`${idPrefix}-amt`}>
            <Input id={`${idPrefix}-amt`} type="number" min={0} value={effect.amount} onChange={(e) => onChange({ ...effect, amount: Number(e.target.value) })} />
          </Field>
          <Field label="Level (for spell slots)" id={`${idPrefix}-lvl`}>
            <Input id={`${idPrefix}-lvl`} type="number" min={0} value={effect.level ?? ""} onChange={(e) => onChange({ ...effect, level: e.target.value ? Number(e.target.value) : undefined })} />
          </Field>
          <div className="md:col-span-3">
            <TargetField value={effect.target} onChange={(t) => onChange({ ...effect, target: t })} idPrefix={idPrefix} />
          </div>
        </div>
      );

    case "set_state":
      return (
        <div className="grid gap-3 md:grid-cols-3">
          <Field label="Entity ID" id={`${idPrefix}-ent`}>
            <Input id={`${idPrefix}-ent`} value={effect.entityId} onChange={(e) => onChange({ ...effect, entityId: e.target.value })} placeholder="ent_npc_…" />
          </Field>
          <Field label="Attribute" id={`${idPrefix}-attr`}>
            <Input id={`${idPrefix}-attr`} value={effect.attribute} onChange={(e) => onChange({ ...effect, attribute: e.target.value })} placeholder="status" />
          </Field>
          <Field label="Value (string)" id={`${idPrefix}-val`}>
            <Input
              id={`${idPrefix}-val`}
              value={String(effect.value ?? "")}
              onChange={(e) => onChange({ ...effect, value: e.target.value })}
              placeholder="dead"
            />
          </Field>
        </div>
      );

    case "move_entity":
      return (
        <div className="grid gap-3 md:grid-cols-2">
          <Field label="Entity ID" id={`${idPrefix}-ent`}>
            <Input id={`${idPrefix}-ent`} value={effect.entityId} onChange={(e) => onChange({ ...effect, entityId: e.target.value })} placeholder="ent_npc_…" />
          </Field>
          <Field label="To location ID" id={`${idPrefix}-loc`}>
            <Input id={`${idPrefix}-loc`} value={effect.toLocationId} onChange={(e) => onChange({ ...effect, toLocationId: e.target.value })} placeholder="ent_location_…" />
          </Field>
        </div>
      );

    case "reveal_entity":
      return (
        <div className="grid gap-3 md:grid-cols-2">
          <Field label="Entity ID" id={`${idPrefix}-ent`}>
            <Input id={`${idPrefix}-ent`} value={effect.entityId} onChange={(e) => onChange({ ...effect, entityId: e.target.value })} />
          </Field>
          <Field label="To users" id={`${idPrefix}-users`}>
            <Select value={typeof effect.toUsers === "string" ? effect.toUsers : "all_players"} onValueChange={(v) => onChange({ ...effect, toUsers: v as "all_players" | "mj_only" })}>
              <SelectTrigger id={`${idPrefix}-users`}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all_players">All players</SelectItem>
                <SelectItem value="mj_only">GM only</SelectItem>
              </SelectContent>
            </Select>
          </Field>
        </div>
      );

    case "set_relation":
      return (
        <div className="grid gap-3 md:grid-cols-3">
          <Field label="From entity" id={`${idPrefix}-from`}>
            <Input id={`${idPrefix}-from`} value={effect.fromId} onChange={(e) => onChange({ ...effect, fromId: e.target.value })} placeholder="ent_…" />
          </Field>
          <Field label="To entity" id={`${idPrefix}-to`}>
            <Input id={`${idPrefix}-to`} value={effect.toId} onChange={(e) => onChange({ ...effect, toId: e.target.value })} />
          </Field>
          <Field label="Delta" id={`${idPrefix}-delta`}>
            <Input id={`${idPrefix}-delta`} type="number" value={effect.delta} onChange={(e) => onChange({ ...effect, delta: Number(e.target.value) })} placeholder="-20" />
          </Field>
          <div className="md:col-span-3">
            <Field label="Disposition (optional)" id={`${idPrefix}-disp`}>
              <Input id={`${idPrefix}-disp`} value={effect.disposition ?? ""} onChange={(e) => onChange({ ...effect, disposition: e.target.value || undefined })} placeholder="ally / enemy / rival" />
            </Field>
          </div>
        </div>
      );

    case "trigger_event":
      return (
        <Field label="Event ID" id={`${idPrefix}-event`}>
          <Input id={`${idPrefix}-event`} value={effect.eventId} onChange={(e) => onChange({ ...effect, eventId: e.target.value })} />
        </Field>
      );

    case "add_to_inventory":
    case "remove_from_inventory":
      return (
        <div className="grid gap-3 md:grid-cols-2">
          <Field label="Item ID" id={`${idPrefix}-item`}>
            <Input id={`${idPrefix}-item`} value={effect.itemId} onChange={(e) => onChange({ ...effect, itemId: e.target.value })} />
          </Field>
          <Field label="Quantity" id={`${idPrefix}-qty`}>
            <Input id={`${idPrefix}-qty`} type="number" min={1} value={effect.quantity ?? 1} onChange={(e) => onChange({ ...effect, quantity: Number(e.target.value) })} />
          </Field>
          <div className="md:col-span-2">
            <TargetField value={effect.target} onChange={(t) => onChange({ ...effect, target: t })} idPrefix={idPrefix} />
          </div>
        </div>
      );

    case "play_ambience":
      return (
        <Field label="Ambience ID" id={`${idPrefix}-amb`}>
          <Input id={`${idPrefix}-amb`} value={effect.ambienceId} onChange={(e) => onChange({ ...effect, ambienceId: e.target.value })} placeholder="ambience_crypt" />
        </Field>
      );

    case "play_music":
      return (
        <Field label="Music ID" id={`${idPrefix}-mus`}>
          <Input id={`${idPrefix}-mus`} value={effect.musicId} onChange={(e) => onChange({ ...effect, musicId: e.target.value })} placeholder="music_combat_tense" />
        </Field>
      );

    case "play_sound":
      return (
        <Field label="Sound ID" id={`${idPrefix}-snd`}>
          <Input id={`${idPrefix}-snd`} value={effect.soundId} onChange={(e) => onChange({ ...effect, soundId: e.target.value })} placeholder="sound_chest_open" />
        </Field>
      );

    case "display_image":
      return (
        <div className="grid gap-3">
          <Field label="Image ID (optional)" id={`${idPrefix}-img`}>
            <Input id={`${idPrefix}-img`} value={effect.imageId ?? ""} onChange={(e) => onChange({ ...effect, imageId: e.target.value || undefined })} />
          </Field>
          <Field label="Or live prompt" id={`${idPrefix}-prompt`}>
            <Input id={`${idPrefix}-prompt`} value={effect.prompt ?? ""} onChange={(e) => onChange({ ...effect, prompt: e.target.value || undefined })} placeholder="Bobby vs the dragon in the cave" />
          </Field>
        </div>
      );

    case "display_text":
      return (
        <Field label="Text" id={`${idPrefix}-text`}>
          <Input id={`${idPrefix}-text`} value={effect.text} onChange={(e) => onChange({ ...effect, text: e.target.value })} placeholder="A cold wind sweeps through the room…" />
        </Field>
      );

    default:
      return null;
  }
}

function Field({ label, id, children }: { label: string; id: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1">
      <Label htmlFor={id} className="text-xs uppercase tracking-wide text-muted-foreground">
        {label}
      </Label>
      {children}
    </div>
  );
}
