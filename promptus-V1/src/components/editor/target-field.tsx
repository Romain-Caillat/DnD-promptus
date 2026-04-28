"use client";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { TARGET_KINDS } from "@/lib/engine/catalog";
import type { TargetSpec } from "@/lib/engine/types";

interface Props {
  value: TargetSpec | undefined;
  onChange: (next: TargetSpec) => void;
  idPrefix?: string;
}

const DEFAULT_TARGET: TargetSpec = { type: "self" };

export function TargetField({ value, onChange, idPrefix = "tgt" }: Props) {
  const target = value ?? DEFAULT_TARGET;

  function setKind(kind: TargetSpec["type"]) {
    if (kind === "single") onChange({ type: "single", entityId: "" });
    else if (kind === "multiple") onChange({ type: "multiple", entityIds: [] });
    else onChange({ type: kind } as TargetSpec);
  }

  return (
    <div className="space-y-2">
      <Label className="text-xs uppercase tracking-wide text-muted-foreground">
        Target
      </Label>
      <Select value={target.type} onValueChange={(v) => setKind(v as TargetSpec["type"])}>
        <SelectTrigger>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {TARGET_KINDS.map((t) => (
            <SelectItem key={t.id} value={t.id}>
              {t.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {target.type === "single" && (
        <div>
          <Label htmlFor={`${idPrefix}-entity-id`} className="text-xs">
            Entity ID
          </Label>
          <Input
            id={`${idPrefix}-entity-id`}
            placeholder="ent_…"
            value={target.entityId}
            onChange={(e) =>
              onChange({ type: "single", entityId: e.target.value })
            }
          />
        </div>
      )}
      {target.type === "multiple" && (
        <div>
          <Label htmlFor={`${idPrefix}-entity-ids`} className="text-xs">
            Entity IDs (comma-separated)
          </Label>
          <Input
            id={`${idPrefix}-entity-ids`}
            placeholder="ent_a, ent_b"
            value={target.entityIds.join(", ")}
            onChange={(e) =>
              onChange({
                type: "multiple",
                entityIds: e.target.value
                  .split(",")
                  .map((s) => s.trim())
                  .filter(Boolean),
              })
            }
          />
        </div>
      )}
    </div>
  );
}
