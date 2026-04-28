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
          <h1 className="text-3xl font-bold">Campaigns</h1>
          <p className="text-muted-foreground mt-1">
            The directories of your worlds — pick one to enter, or start a new saga.
          </p>
        </div>
        <CreateCampaignButton />
      </header>

      {rows.length === 0 ? (
        <Card className="text-center py-16">
          <CardHeader>
            <CardTitle>No campaign yet</CardTitle>
            <CardDescription>Create your first campaign to begin.</CardDescription>
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
                  <Link href={`/campaigns/${c.id}/entities`}>Entities</Link>
                </Button>
                <Button asChild size="sm" variant="outline">
                  <Link href={`/campaigns/${c.id}`}>Open</Link>
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </main>
  );
}
