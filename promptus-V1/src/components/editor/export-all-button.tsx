"use client";

import { Button } from "@/components/ui/button";

export function ExportAllButton({
  campaignId,
  disabled,
}: {
  campaignId: string;
  disabled?: boolean;
}) {
  return (
    <Button asChild variant="outline" disabled={disabled}>
      <a href={`/api/campaigns/${campaignId}/entities/export`} download>
        Tout exporter
      </a>
    </Button>
  );
}
