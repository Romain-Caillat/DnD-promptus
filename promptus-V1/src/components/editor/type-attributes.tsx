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
import { Button } from "@/components/ui/button";
import { useRuleset } from "@/components/providers/ruleset-provider";
import { SCHOOLS_OF_MAGIC, SCHOOL_LABELS } from "@/lib/engine/catalog";
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
          <NumField label="Niveau du sort" name="level" attributes={attributes} set={set} placeholder="3" />
          <SelectField label="École" name="school" options={SCHOOLS_OF_MAGIC} labels={SCHOOL_LABELS} attributes={attributes} set={set} />
          <TextField label="Temps d’incantation" name="castingTime" attributes={attributes} set={set} placeholder="1 action" />
          <TextField label="Portée" name="range" attributes={attributes} set={set} placeholder="45 m" />
          <TextField label="Durée" name="duration" attributes={attributes} set={set} placeholder="instantanée" />
          <TextField label="Composantes (V/S/M)" name="components" attributes={attributes} set={set} placeholder="V, S, M" parseList />
        </div>
      );

    case "monster":
    case "character":
      return (
        <div className="grid gap-3 md:grid-cols-3">
          <NumField label="PV" name="hp" attributes={attributes} set={set} />
          <NumField label="PV max" name="hpMax" attributes={attributes} set={set} />
          <NumField label="Classe d’armure" name="ac" attributes={attributes} set={set} />
          <NumField label="Vitesse (pieds)" name="speed" attributes={attributes} set={set} />
          <NumField label="Bonus d’initiative" name="initiativeBonus" attributes={attributes} set={set} />
          <NumField label="Facteur de puissance" name="challengeRating" attributes={attributes} set={set} step="0.25" />
          <TextField label="Taille" name="size" attributes={attributes} set={set} placeholder="Moyenne" />
          <TextField label="Alignement" name="alignment" attributes={attributes} set={set} placeholder="Neutre mauvais" />
          <NumField label="Niveau" name="level" attributes={attributes} set={set} />
          <div className="md:col-span-3">
            <AttacksField attributes={attributes} set={set} />
          </div>
        </div>
      );

    case "npc":
      return (
        <div className="grid gap-3 md:grid-cols-2">
          <NumField label="PV" name="hp" attributes={attributes} set={set} />
          <NumField label="PV max" name="hpMax" attributes={attributes} set={set} />
          <NumField label="Classe d’armure" name="ac" attributes={attributes} set={set} />
          <TextField label="Faction" name="faction" attributes={attributes} set={set} />
          <TextField label="Lieu actuel" name="currentLocation" attributes={attributes} set={set} />
          <SelectField
            label="Statut"
            name="status"
            options={["alive", "wounded", "dead", "missing"]}
            labels={{ alive: "vivant", wounded: "blessé", dead: "mort", missing: "disparu" }}
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
              Secret (MJ uniquement)
            </Label>
            <Textarea
              value={(attributes.secret as string) ?? ""}
              onChange={(e) => set("secret", e.target.value)}
              rows={2}
            />
          </div>
          <TextField label="Note d’interprétation (voix)" name="voiceActorRecommended" attributes={attributes} set={set} />
          <div className="md:col-span-2">
            <AttacksField attributes={attributes} set={set} />
          </div>
        </div>
      );

    case "item":
      return (
        <div className="grid gap-3 md:grid-cols-3">
          <SelectField
            label="Rareté"
            name="rarity"
            options={["common", "uncommon", "rare", "very_rare", "legendary"]}
            labels={{ common: "commun", uncommon: "peu commun", rare: "rare", very_rare: "très rare", legendary: "légendaire" }}
            attributes={attributes}
            set={set}
          />
          <NumField label="Poids (lb)" name="weight" attributes={attributes} set={set} step="0.1" />
          <NumField label="Valeur (po)" name="value" attributes={attributes} set={set} />
          <BoolField label="Harmonisation requise" name="attunement" attributes={attributes} set={set} />
        </div>
      );

    case "location":
      return (
        <div className="grid gap-3">
          <TextField label="Lieu parent" name="parentLocation" attributes={attributes} set={set} placeholder="ent_location_…" />
          <TextField label="ID de l’ambiance par défaut" name="defaultAmbienceId" attributes={attributes} set={set} placeholder="ambience_crypt" />
        </div>
      );

    case "faction":
      return (
        <div className="grid gap-3">
          <NumField label="Réputation" name="reputation" attributes={attributes} set={set} />
        </div>
      );

    case "event":
    case "condition":
    default:
      return (
        <p className="text-sm text-muted-foreground italic">
          Pas d’attributs spécifiques pour ce type. Utilisez la description et les effets ci-dessous.
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
  labels,
  attributes,
  set,
}: {
  label: string;
  name: string;
  options: readonly string[];
  labels?: Record<string, string>;
  attributes: Record<string, unknown>;
  set: (k: string, v: unknown) => void;
}) {
  return (
    <div className="space-y-1">
      <Label className="text-xs uppercase tracking-wide text-muted-foreground">{label}</Label>
      <Select value={(attributes[name] as string) ?? ""} onValueChange={(v) => set(name, v)}>
        <SelectTrigger>
          <SelectValue placeholder="Choisir…" />
        </SelectTrigger>
        <SelectContent>
          {options.map((o) => (
            <SelectItem key={o} value={o}>
              {labels?.[o] ?? o}
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

interface AttackRow {
  name: string;
  bonus?: number;
  damage: string;
  damageType?: string;
  rangeMeters?: number;
  longRangeMeters?: number;
}

/** Attaques de la créature : utilisées en combat sur la carte (bonus, dégâts, portée). */
function AttacksField({ attributes, set }: { attributes: Record<string, unknown>; set: (k: string, v: unknown) => void }) {
  const ruleset = useRuleset();
  const rows = (Array.isArray(attributes.attacks) ? attributes.attacks : []) as AttackRow[];
  const update = (i: number, patch: Partial<AttackRow>) =>
    set("attacks", rows.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const numOrUndef = (v: string) => (v === "" ? undefined : Number(v));
  return (
    <div className="space-y-2" data-testid="attacks-field">
      <Label className="text-xs uppercase tracking-wide text-muted-foreground">Attaques</Label>
      {rows.map((r, i) => (
        <div key={i} className="grid gap-2 grid-cols-2 md:grid-cols-[2fr_70px_1fr_1.3fr_80px_80px_auto] items-center">
          <Input aria-label="Nom de l’attaque" placeholder="Épée longue" value={r.name ?? ""} onChange={(e) => update(i, { name: e.target.value })} />
          <Input aria-label="Bonus" type="number" placeholder="+5" value={r.bonus ?? ""} onChange={(e) => update(i, { bonus: numOrUndef(e.target.value) })} />
          <Input aria-label="Dégâts" placeholder="1d8+3" value={r.damage ?? ""} onChange={(e) => update(i, { damage: e.target.value })} />
          <Select value={r.damageType ?? ""} onValueChange={(v) => update(i, { damageType: v })}>
            <SelectTrigger aria-label="Type de dégâts">
              <SelectValue placeholder="Type" />
            </SelectTrigger>
            <SelectContent>
              {ruleset.damageTypes.map((d) => (
                <SelectItem key={d.id} value={d.id}>
                  {d.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            aria-label="Portée (m)"
            type="number"
            step="1.5"
            placeholder="1,5 m"
            value={r.rangeMeters ?? ""}
            onChange={(e) => update(i, { rangeMeters: numOrUndef(e.target.value) })}
          />
          <Input
            aria-label="Portée longue (m)"
            type="number"
            step="1.5"
            placeholder="longue"
            value={r.longRangeMeters ?? ""}
            onChange={(e) => update(i, { longRangeMeters: numOrUndef(e.target.value) })}
          />
          <Button type="button" size="sm" variant="ghost" onClick={() => set("attacks", rows.filter((_, j) => j !== i))}>
            Retirer
          </Button>
        </div>
      ))}
      <Button
        type="button"
        size="sm"
        variant="outline"
        onClick={() => set("attacks", [...rows, { name: "", bonus: 0, damage: "1d6", damageType: ruleset.damageTypes[0]?.id, rangeMeters: 1.5 }])}
      >
        Ajouter une attaque
      </Button>
      <p className="text-xs text-muted-foreground">Portée en mètres : 1,5 m = au contact. Au-delà de la portée normale et jusqu’à la longue : désavantage.</p>
    </div>
  );
}
