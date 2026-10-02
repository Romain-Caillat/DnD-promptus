import Link from "next/link";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { desc } from "drizzle-orm";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { CreateCampaignButton } from "@/components/campaigns/create-campaign-button";

export const dynamic = "force-dynamic";

export default async function CampaignsPage() {
  const rows = await db.select().from(campaigns).orderBy(desc(campaigns.updatedAt));

  return (
    <main className="flex-1 px-6 py-12 max-w-6xl mx-auto w-full">
      <header className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-3xl font-bold">Campagnes</h1>
          <p className="text-muted-foreground mt-1">
            Vos mondes : ouvrez-en un, ou lancez une nouvelle saga.
          </p>
        </div>
        <CreateCampaignButton />
      </header>

      {rows.length === 0 ? (
        <Card className="text-center py-16">
          <CardHeader>
            <CardTitle>Aucune campagne</CardTitle>
            <CardDescription>Créez votre première campagne pour commencer.</CardDescription>
          </CardHeader>
          <CardContent>
            <CreateCampaignButton />
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {rows.map((c) => (
            <Card key={c.id} className="hover:border-primary transition-colors">
              <CardHeader>
                <CardTitle>{c.name}</CardTitle>
                {c.description ? (
                  <CardDescription>{c.description}</CardDescription>
                ) : null}
              </CardHeader>
              <CardContent className="flex gap-2">
                <Button asChild size="sm" variant="default">
                  <Link href={`/campaigns/${c.id}/entities`}>Fiches</Link>
                </Button>
                <Button asChild size="sm" variant="outline">
                  <Link href={`/campaigns/${c.id}`}>Ouvrir</Link>
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </main>
  );
}
