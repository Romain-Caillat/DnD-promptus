"use client";

import { useState } from "react";
import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { EntityType } from "@/lib/engine/types";
import type { EntityRow } from "@/lib/db/schema";
import { ImportYamlDialog } from "@/components/editor/import-yaml-dialog";
import { ExportAllButton } from "@/components/editor/export-all-button";

const TYPES: { id: "all" | EntityType; label: string }[] = [
  { id: "all", label: "All" },
  { id: "spell", label: "Spells" },
  { id: "item", label: "Items" },
  { id: "npc", label: "NPCs" },
  { id: "monster", label: "Monsters" },
  { id: "character", label: "Characters" },
  { id: "location", label: "Locations" },
  { id: "event", label: "Events" },
  { id: "condition", label: "Conditions" },
];

export function EntitiesView({ campaignId }: { campaignId: string }) {
  const [type, setType] = useState<"all" | EntityType>("all");

  const { data, isLoading, error } = useQuery({
    queryKey: ["entities", campaignId, type],
    queryFn: async () => {
      const url =
        type === "all"
          ? `/api/campaigns/${campaignId}/entities`
          : `/api/campaigns/${campaignId}/entities?type=${type}`;
      const res = await fetch(url);
      if (!res.ok) throw new Error("Failed to load entities");
      return res.json() as Promise<{ entities: EntityRow[] }>;
    },
  });

  const entities = data?.entities ?? [];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between gap-4 flex-wrap">
        <Tabs value={type} onValueChange={(v) => setType(v as typeof type)}>
          <TabsList>
            {TYPES.map((t) => (
              <TabsTrigger key={t.id} value={t.id}>
                {t.label}
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
        <div className="flex gap-2">
          <ExportAllButton campaignId={campaignId} disabled={entities.length === 0} />
          <ImportYamlDialog campaignId={campaignId} />
          <Button asChild>
            <Link href={`/campaigns/${campaignId}/entities/new`}>Create entity</Link>
          </Button>
        </div>
      </div>

      {error ? (
        <Card>
          <CardContent className="py-8 text-center text-destructive">
            {(error as Error).message}
          </CardContent>
        </Card>
      ) : isLoading ? (
        <Card>
          <CardContent className="py-8 text-center text-muted-foreground">
            Loading entities…
          </CardContent>
        </Card>
      ) : entities.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-muted-foreground">
            No {type === "all" ? "entities" : `${type}s`} yet.
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3" data-testid="entity-grid">
          {entities.map((e) => (
            <Link
              key={e.id}
              href={`/campaigns/${campaignId}/entities/${e.id}/edit`}
              className="block"
            >
              <Card className="hover:border-primary transition-colors h-full">
                <CardHeader>
                  <div className="flex items-start justify-between gap-2">
                    <CardTitle className="text-lg">{e.name}</CardTitle>
                    <Badge variant="outline" className="capitalize">
                      {e.type}
                    </Badge>
                  </div>
                  {e.description ? (
                    <CardDescription className="line-clamp-2">{e.description}</CardDescription>
                  ) : null}
                </CardHeader>
                <CardContent className="flex flex-wrap gap-1">
                  {e.tags.slice(0, 4).map((tag) => (
                    <Badge key={tag} variant="secondary">
                      {tag}
                    </Badge>
                  ))}
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
