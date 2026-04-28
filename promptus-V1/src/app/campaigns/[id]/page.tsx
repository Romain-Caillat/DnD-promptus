import { notFound } from "next/navigation";
import Link from "next/link";
import { db } from "@/lib/db/client";
import { campaigns } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { StartSessionDialog } from "@/components/sessions/start-session-dialog";
import { SessionsList } from "@/components/sessions/sessions-list";

export const dynamic = "force-dynamic";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function CampaignDashboardPage({ params }: Props) {
  const { id } = await params;
  const [campaign] = await db.select().from(campaigns).where(eq(campaigns.id, id));
  if (!campaign) notFound();

  return (
    <main className="flex-1 px-6 py-8 max-w-6xl mx-auto w-full">
      <Button asChild variant="ghost" size="sm" className="mb-2 -ml-2">
        <Link href="/campaigns">← Campaigns</Link>
      </Button>
      <div className="flex items-start justify-between gap-4 flex-wrap">
        <div>
          <h1 className="text-3xl font-bold">{campaign.name}</h1>
          {campaign.description ? (
            <p className="text-muted-foreground mt-1">{campaign.description}</p>
          ) : null}
        </div>
        <Button asChild variant="outline" size="sm">
          <Link href={`/campaigns/${campaign.id}/settings`}>Settings</Link>
        </Button>
      </div>

      <section className="mt-8">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-semibold">Sessions</h2>
          <StartSessionDialog campaignId={campaign.id} />
        </div>
        <SessionsList campaignId={campaign.id} />
      </section>

      <section className="mt-8">
        <h2 className="text-xl font-semibold mb-4">World & assets</h2>
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          <Link href={`/campaigns/${campaign.id}/entities`}>
            <Card className="hover:border-primary transition-colors h-full">
              <CardHeader>
                <CardTitle className="text-base">Entities</CardTitle>
                <CardDescription>
                  Spells, items, NPCs, monsters, locations and events.
                </CardDescription>
              </CardHeader>
            </Card>
          </Link>
          <Link href={`/campaigns/${campaign.id}/audio`}>
            <Card className="hover:border-primary transition-colors h-full">
              <CardHeader>
                <CardTitle className="text-base">Audio library</CardTitle>
                <CardDescription>
                  Ambiences, music and sound effects available in cockpit.
                </CardDescription>
              </CardHeader>
            </Card>
          </Link>
          <Card className="opacity-50 cursor-not-allowed">
            <CardHeader>
              <CardTitle className="text-base">Replay archive</CardTitle>
              <CardDescription>Available in V1.</CardDescription>
            </CardHeader>
          </Card>
        </div>
      </section>
    </main>
  );
}
