import { notFound } from "next/navigation";
import Link from "next/link";
import { db } from "@/lib/db/client";
import { campaigns, entities } from "@/lib/db/schema";
import { and, eq } from "drizzle-orm";
import { Button } from "@/components/ui/button";
import { RulesetProvider } from "@/components/providers/ruleset-provider";
import { EntityEditor, type EntityFormState } from "@/components/editor/entity-editor";
import { ENTITY_TYPE_LABELS } from "@/lib/engine/catalog";

export const dynamic = "force-dynamic";

interface Props {
  params: Promise<{ id: string; entityId: string }>;
}

export default async function EditEntityPage({ params }: Props) {
  const { id, entityId } = await params;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, id));
  if (!campaign) notFound();

  const [entity] = await db
    .select()
    .from(entities)
    .where(and(eq(entities.id, entityId), eq(entities.campaignId, id)));
  if (!entity) notFound();

  const initial: EntityFormState = {
    id: entity.id,
    type: entity.type,
    name: entity.name,
    description: entity.description ?? "",
    imageUrl: entity.imageUrl ?? "",
    tags: entity.tags ?? [],
    attributes: entity.attributes ?? {},
    effects: entity.effects ?? [],
    visibility: entity.visibility,
  };

  return (
    <main className="flex-1 px-6 py-8 max-w-5xl mx-auto w-full">
      <Button asChild variant="ghost" size="sm" className="mb-2 -ml-2">
        <Link href={`/campaigns/${id}/entities`}>← Fiches</Link>
      </Button>
      <h1 className="text-3xl font-bold mb-1">Modifier {entity.name}</h1>
      <p className="text-muted-foreground mb-8">
        {ENTITY_TYPE_LABELS[entity.type]} · version {entity.version}
      </p>

      <RulesetProvider campaignId={id}>
        <EntityEditor campaignId={id} mode="edit" initial={initial} />
      </RulesetProvider>
    </main>
  );
}
