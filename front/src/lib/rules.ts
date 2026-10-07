import { apiRequest } from './api'
import type { ActionCardView } from './play'

/** The four outcome bands of a roll (`rules::check::OutcomeBand`). */
export type OutcomeBand = 'critical_failure' | 'failure' | 'success' | 'critical_success'

/** Where a modifier comes from (`rules::check::ModifierSource`). */
export type ModifierSource =
  | { from: 'ability'; id: string }
  | { from: 'precision'; id: string }
  | { from: 'condition'; id: string }
  | { from: 'situation'; id: string }
  | { from: 'cover'; id: string }
  | { from: 'long_range' }
  | { from: 'attack_bonus'; id: string }

interface Modifier {
  source: ModifierSource
  value: number
}

export type RollTarget =
  | { against: 'difficulty'; id: string | null; value: number }
  | { against: 'armor_class'; value: number }
  | { against: 'opposed'; value: number }

/**
 * One roll as the rules engine makes it (`rules::check::RollBreakdown`):
 * the faces, the face kept, every modifier and where it comes from, the
 * total, the target when known, and the band.
 */
export interface RollBreakdown {
  die: string
  faces: number[]
  natural: number
  advantage: 'normal' | 'advantage' | 'disadvantage'
  modifiers: Modifier[]
  total: number
  target: RollTarget | null
  /** `null` while no target is known and the natural face does not decide. */
  band: OutcomeBand | null
}

interface OutcomeView {
  band: OutcomeBand
  name: string
  description: string
  natural: number[]
  xp: number
  damageMultiplier: number | null
}

interface AbilityView {
  id: string
  name: string
  description: string
  score: number | null
  modifier: number | null
  needs: { difficulty: string; fromFace: number | null }[]
}

interface DifficultyView {
  id: string
  name: string
  value: number
  description: string
}

interface StatView {
  name: string
  abbr: string
  formula: string
}

interface KindView {
  name: string
  description: string
  cost: number
}

/** A card as the rules page explains it (`projection::rules::RuleCardView`). */
export interface RuleCardView extends ActionCardView {
  cost: number
  attackModifiers: Modifier[] | null
}

export type ChangeValue =
  | { kind: 'number'; value: number }
  | { kind: 'text'; value: string }
  | { kind: 'flag'; value: boolean }
  | { kind: 'code'; value: string }

/** One change between two versions (`rules::changes::RuleChange`). */
export interface RuleChange {
  section: string
  subject: string
  field: string
  from: ChangeValue | null
  to: ChangeValue | null
}

/** The campaign's rules as one page (`projection::rules::RulesView`). */
export interface RulesView {
  name: string
  version: number
  check: { die: string; faces: number; advantage: boolean; modifier: string }
  abilities: AbilityView[]
  difficulties: DifficultyView[]
  outcomes: OutcomeView[]
  attack: {
    ability: 'first_primary' | 'best_primary'
    precisionApplies: boolean
    /** Added to every attack with the level (the SRD's proficiency). */
    bonus: { name: string; formula: string } | null
    armorClass: StatView
  }
  hitPoints: StatView
  turns: { name: string; actionsPerTurn: number; limits: { kind: string; maxPerTurn: number }[] }[]
  actionKinds: KindView[]
  cooldown: 'skip_next_turns' | 'turn_of_use_counts'
  applicationTurnCounts: boolean
  conditions: { name: string; description: string; kind: 'boon' | 'bane' }[]
  zeroHp:
    | { rule: 'knockedOut'; condition: string; outAfterTurns: number; outCondition: string }
    | { rule: 'deathSaves'; difficulty: number; successes: number; failures: number }
  progression: { upgradeEveryXp: number; upgradePoints: number; levels: { level: number; xp: number }[] }
  combat: {
    moveKind: KindView | null
    flee: { kind: KindView; ability: string | null } | null
    coverHalf: number
    coverThreeQuarters: number
    longRangeDisadvantage: boolean
    longRangeModifier: number | null
  }
  groupCheck: 'at_least_half' | 'majority' | 'all' | 'any' | null
  houseRules: { name: string; text: string }[]
  mine: {
    className: string
    cards: RuleCardView[]
    exampleAbility: string
    exampleDifficulty: string
    examples: RollBreakdown[]
  } | null
  changes: { fromVersion: number; toVersion: number; replaced: boolean; items: RuleChange[] } | null
}

/** `GET /api/play/{campaign}/rules`. */
export function fetchRules(campaignId: string): Promise<RulesView> {
  return apiRequest<RulesView>('GET', `/play/${encodeURIComponent(campaignId)}/rules`)
}

/** `POST /api/play/{campaign}/rules/seen`: the player read this version. */
export function markRulesSeen(campaignId: string): Promise<RulesView> {
  return apiRequest<RulesView>('POST', `/play/${encodeURIComponent(campaignId)}/rules/seen`)
}

/** Where the rules page of `campaignId` lives in the app. */
export function rulesPath(campaignId: string): string {
  return `/partie/${encodeURIComponent(campaignId)}/regles`
}
