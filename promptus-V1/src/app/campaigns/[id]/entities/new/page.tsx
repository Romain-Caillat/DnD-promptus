import { notFound } from "next/navigation";
import Link from "next/link";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { Button } from "@/components/ui/button";
import { EntityEditor } from "@/components/editor/entity-editor";

export const dynamic = "force-dynamic";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function NewEntityPage({ params }: Props) {
  const { id } = await params;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, id));
  if (!campaign) notFound();

  return (
    <main className="flex-1 px-6 py-8 max-w-5xl mx-auto w-full">
      <Button asChild variant="ghost" size="sm" className="mb-2 -ml-2">
        <Link href={`/campaigns/${id}/entities`}>← Entities</Link>
      </Button>
      <h1 className="text-3xl font-bold mb-1">Create entity</h1>
      <p className="text-muted-foreground mb-8">
        Declare a new piece of your world — spell, NPC, item, location…
      </p>

      <EntityEditor campaignId={id} mode="create" />
    </main>
  );
}
