"use client";

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
import { SCHOOLS_OF_MAGIC } from "@/lib/engine/catalog";
import type { EntityType } from "@/lib/engine/types";

interface Props {
  type: EntityType;
  attributes: Record<string, unknown>;
  onChange: (next: Record<string, unknown>) => void;
}

/**
 * Type-specific attribute fields. Renders the relevant subset of fields based
 * on entity type. Stores everything in the generic `attributes` record so the
 * engine can read it without coupling to the form.
 */
export function TypeAttributes({ type, attributes, onChange }: Props) {
  function set<K extends string>(key: K, value: unknown) {
    onChange({ ...attributes, [key]: value });
  }

  switch (type) {
    case "spell":
      return (
        <div className="grid gap-3 md:grid-cols-2">
          <NumField label="Spell level" name="level" attributes={attributes} set={set} placeholder="3" />
          <SelectField label="School" name="school" options={SCHOOLS_OF_MAGIC} attributes={attributes} set={set} />
          <TextField label="Casting time" name="castingTime" attributes={attributes} set={set} placeholder="1 action" />
          <TextField label="Range" name="range" attributes={attributes} set={set} placeholder="150 ft" />
          <TextField label="Duration" name="duration" attributes={attributes} set={set} placeholder="instantaneous" />
          <TextField label="Components (V/S/M)" name="components" attributes={attributes} set={set} placeholder="V, S, M" parseList />
        </div>
      );

    case "monster":
    case "character":
      return (
        <div className="grid gap-3 md:grid-cols-3">
          <NumField label="HP" name="hp" attributes={attributes} set={set} />
          <NumField label="HP max" name="hpMax" attributes={attributes} set={set} />
          <NumField label="Armor Class" name="ac" attributes={attributes} set={set} />
          <NumField label="Speed (ft)" name="speed" attributes={attributes} set={set} />
          <NumField label="Initiative bonus" name="initiativeBonus" attributes={attributes} set={set} />
          <NumField label="Challenge rating" name="challengeRating" attributes={attributes} set={set} step="0.25" />
          <TextField label="Size" name="size" attributes={attributes} set={set} placeholder="Medium" />
          <TextField label="Alignment" name="alignment" attributes={attributes} set={set} placeholder="Neutral Evil" />
          <NumField label="Level" name="level" attributes={attributes} set={set} />
        </div>
      );

    case "npc":
      return (
        <div className="grid gap-3 md:grid-cols-2">
          <NumField label="HP" name="hp" attributes={attributes} set={set} />
          <NumField label="HP max" name="hpMax" attributes={attributes} set={set} />
          <NumField label="Armor Class" name="ac" attributes={attributes} set={set} />
          <TextField label="Faction" name="faction" attributes={attributes} set={set} />
          <TextField label="Current location" name="currentLocation" attributes={attributes} set={set} />
          <SelectField
            label="Status"
            name="status"
            options={["alive", "wounded", "dead", "missing"]}
            attributes={attributes}
            set={set}
          />
          <div className="md:col-span-2 space-y-1">
            <Label className="text-xs uppercase tracking-wide text-muted-foreground">
              Motivation
            </Label>
            <Textarea
              value={(attributes.motivation as string) ?? ""}
              onChange={(e) => set("motivation", e.target.value)}
              rows={2}
            />
          </div>
          <div className="md:col-span-2 space-y-1">
            <Label className="text-xs uppercase tracking-wide text-muted-foreground">
              Secret (GM only)
            </Label>
            <Textarea
              value={(attributes.secret as string) ?? ""}
              onChange={(e) => set("secret", e.target.value)}
              rows={2}
            />
          </div>
          <TextField label="Voice acting note" name="voiceActorRecommended" attributes={attributes} set={set} />
        </div>
      );

    case "item":
      return (
        <div className="grid gap-3 md:grid-cols-3">
          <SelectField
            label="Rarity"
            name="rarity"
            options={["common", "uncommon", "rare", "very_rare", "legendary"]}
            attributes={attributes}
            set={set}
          />
          <NumField label="Weight (lb)" name="weight" attributes={attributes} set={set} step="0.1" />
          <NumField label="Value (gp)" name="value" attributes={attributes} set={set} />
          <BoolField label="Requires attunement" name="attunement" attributes={attributes} set={set} />
        </div>
      );

    case "location":
      return (
        <div className="grid gap-3">
          <TextField label="Parent location" name="parentLocation" attributes={attributes} set={set} placeholder="ent_location_…" />
          <TextField label="Default ambience ID" name="defaultAmbienceId" attributes={attributes} set={set} placeholder="ambience_crypt" />
        </div>
      );

    case "faction":
      return (
        <div className="grid gap-3">
          <NumField label="Reputation" name="reputation" attributes={attributes} set={set} />
        </div>
      );

    case "event":
    case "condition":
    default:
      return (
        <p className="text-sm text-muted-foreground italic">
          No type-specific attributes for {type}. Use the description and effects below.
        </p>
      );
  }
}

// ----------------------------------------------------------------------------
// Field helpers
// ----------------------------------------------------------------------------

function TextField({
  label,
  name,
  attributes,
  set,
  placeholder,
  parseList,
}: {
  label: string;
  name: string;
  attributes: Record<string, unknown>;
  set: (k: string, v: unknown) => void;
  placeholder?: string;
  parseList?: boolean;
}) {
  const raw = attributes[name];
  const value = parseList && Array.isArray(raw) ? raw.join(", ") : (raw as string) ?? "";
  return (
    <div className="space-y-1">
      <Label className="text-xs uppercase tracking-wide text-muted-foreground">{label}</Label>
      <Input
        value={value}
        onChange={(e) =>
          set(name, parseList ? e.target.value.split(",").map((s) => s.trim()).filter(Boolean) : e.target.value)
        }
        placeholder={placeholder}
      />
    </div>
  );
}

function NumField({
  label,
  name,
  attributes,
  set,
  placeholder,
  step,
}: {
  label: string;
  name: string;
  attributes: Record<string, unknown>;
  set: (k: string, v: unknown) => void;
  placeholder?: string;
  step?: string;
}) {
  return (
    <div className="space-y-1">
      <Label className="text-xs uppercase tracking-wide text-muted-foreground">{label}</Label>
      <Input
        type="number"
        step={step}
        value={(attributes[name] as number | undefined) ?? ""}
        onChange={(e) => set(name, e.target.value ? Number(e.target.value) : undefined)}
        placeholder={placeholder}
      />
    </div>
  );
}

function SelectField({
  label,
  name,
  options,
  attributes,
  set,
}: {
  label: string;
  name: string;
  options: readonly string[];
  attributes: Record<string, unknown>;
  set: (k: string, v: unknown) => void;
}) {
  return (
    <div className="space-y-1">
      <Label className="text-xs uppercase tracking-wide text-muted-foreground">{label}</Label>
      <Select value={(attributes[name] as string) ?? ""} onValueChange={(v) => set(name, v)}>
        <SelectTrigger>
          <SelectValue placeholder="Choose…" />
        </SelectTrigger>
        <SelectContent>
          {options.map((o) => (
            <SelectItem key={o} value={o}>
              {o}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

function BoolField({
  label,
  name,
  attributes,
  set,
}: {
  label: string;
  name: string;
  attributes: Record<string, unknown>;
  set: (k: string, v: unknown) => void;
}) {
  return (
    <div className="flex items-center gap-2 pt-6">
      <input
        type="checkbox"
        checked={Boolean(attributes[name])}
        onChange={(e) => set(name, e.target.checked)}
        className="size-4"
      />
      <Label className="text-sm">{label}</Label>
    </div>
  );
}
