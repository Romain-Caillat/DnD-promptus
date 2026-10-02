"use client";

import { createContext, useContext } from "react";
import { useQuery } from "@tanstack/react-query";
import { DND5E_RULESET, type Ruleset } from "@/lib/engine/ruleset";

const RulesetContext = createContext<Ruleset>(DND5E_RULESET);

export function rulesetQueryKey(campaignId: string) {
  return ["ruleset", campaignId] as const;
}

/** Charge les règles de la campagne et les rend disponibles via `useRuleset()`. */
export function RulesetProvider({
  campaignId,
  children,
}: {
  campaignId: string;
  children: React.ReactNode;
}) {
  const { data } = useQuery({
    queryKey: rulesetQueryKey(campaignId),
    queryFn: async () => {
      const res = await fetch(`/api/campaigns/${campaignId}/rules`);
      if (!res.ok) throw new Error("Impossible de charger les règles");
      return (await res.json()) as { ruleset: Ruleset; isPreset: boolean };
    },
    staleTime: 60_000,
  });
  return (
    <RulesetContext.Provider value={data?.ruleset ?? DND5E_RULESET}>
      {children}
    </RulesetContext.Provider>
  );
}

export function useRuleset(): Ruleset {
  return useContext(RulesetContext);
}
