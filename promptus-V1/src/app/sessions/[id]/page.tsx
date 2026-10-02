import { notFound } from "next/navigation";
import { db } from "@/lib/db/client";
import { sessions } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { Cockpit } from "@/components/cockpit/cockpit";
import { RulesetProvider } from "@/components/providers/ruleset-provider";

export const dynamic = "force-dynamic";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function SessionCockpitPage({ params }: Props) {
  const { id } = await params;
  const [session] = await db.select().from(sessions).where(eq(sessions.id, id));
  if (!session) notFound();

  return (
    <RulesetProvider campaignId={session.campaignId}>
      <Cockpit sessionId={session.id} campaignId={session.campaignId} />
    </RulesetProvider>
  );
}
