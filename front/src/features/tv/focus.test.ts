import { describe, expect, it } from 'vitest'
import type { ScreenView } from '@/lib/screens'
import { freshHighlights, newHits, pickFocus } from './focus'

const NOW = Date.parse('2026-10-10T20:30:00Z')
const ago = (s: number) => new Date(NOW - s * 1000).toISOString()

function view(over: Partial<ScreenView> = {}): ScreenView {
  return {
    title: 'Le Brasier',
    shows: { scene: true, map: true, party: true, moments: true },
    session: { number: 4, status: 'live', startedAt: ago(600) },
    previously: null,
    music: null,
    party: [],
    scene: null,
    board: null,
    moments: [],
    rolls: [],
    tonight: [],
    ...over,
  }
}

const roll = (id: string, at: string) => ({
  id,
  character: 'Kaël',
  ability: 'Dextérité',
  difficulty: 'Moyen',
  roll: {
    die: '1d20',
    faces: [16],
    natural: 16,
    advantage: 'normal' as const,
    modifiers: [],
    total: 18,
    target: null,
    band: 'success' as const,
  },
  outcome: 'Réussite',
  at,
})

const board = (live: boolean) =>
  ({ map: {}, fog: false, tokens: [], reachable: [], fight: live ? { live: true } : null }) as unknown as ScreenView['board']
const scene = { id: 'sc_1', act: 'a1', title: 'La passerelle', readAloud: '', place: null, npcs: [], opponents: [] }

describe('pickFocus', () => {
  it('follows the evening: lobby, « Précédemment… », scene, map, fight, then the end', () => {
    expect(pickFocus(view({ session: null }))).toBe('between')
    expect(pickFocus(view({ session: { number: 4, status: 'lobby', startedAt: null } }))).toBe('lobby')
    expect(pickFocus(view({ previously: 'Le Cure-Dent a sauté.' }))).toBe('previously')
    expect(pickFocus(view({ previously: 'Le Cure-Dent a sauté.', scene }))).toBe('scene')
    expect(pickFocus(view({ scene, board: board(false) }))).toBe('map')
    expect(pickFocus(view({ scene, board: board(true) }))).toBe('fight')
    expect(pickFocus(view())).toBe('waiting')
  })
})

describe('freshHighlights', () => {
  it('plays new rolls and finds in order, never a stale or seen one, never a scene line', () => {
    const v = view({
      rolls: [roll('r-old', ago(300)), roll('r-new', ago(5))],
      moments: [
        { id: 'm-clue', kind: 'clue', text: 'Un émetteur actif.', at: ago(10) },
        { id: 'm-scene', kind: 'scene', text: 'La passerelle', at: ago(8) },
        { id: 'm-roll', kind: 'roll', text: 'Kaël — Dextérité 18', at: ago(5) },
        { id: 'm-loot', kind: 'loot', text: 'Une plaque de chitine', at: ago(2) },
      ],
    })
    expect(freshHighlights(v, new Set(), NOW).map((h) => h.id)).toEqual(['m-clue', 'r-new', 'm-loot'])
    // A moment already played is not played again.
    expect(freshHighlights(v, new Set(['m-clue', 'r-new']), NOW).map((h) => h.id)).toEqual(['m-loot'])
  })
})

const damaged = (target: string, before: number, after: number, total: number) => ({
  kind: 'rules' as const,
  event: { event: 'damaged' as const, target, breakdown: { total }, hp_before: before, hp_after: after },
})

const fightOf = (events: unknown[]) =>
  view({
    board: {
      map: {},
      fog: false,
      tokens: [],
      reachable: [],
      fight: {
        live: true,
        order: [
          { id: 'pc_kael', name: 'Kaël' },
          { id: 'foe_1', name: 'Pillard' },
        ],
        events,
      },
    } as unknown as ScreenView['board'],
  })

describe('newHits', () => {
  it('plays each blow once, from the first view on, and starts over with a new fight', () => {
    const opening = [{ kind: 'round_started', round: 1 }, damaged('foe_1', 0, 0, 6)]
    // A TV opening mid-fight replays nothing.
    const first = newHits(fightOf(opening), null, NOW)
    expect(first.hits).toEqual([])
    // A blow on a foe reads the damage rolled (its hit points are hidden);
    // one on the party reads what was lost, and says when it goes down.
    const log = [
      ...opening,
      damaged('foe_1', 0, 0, 4),
      damaged('pc_kael', 3, 0, 9),
      { kind: 'rules', event: { event: 'knocked_out', target: 'pc_kael' } },
      { kind: 'rules', event: { event: 'missed', target: 'foe_1' } },
    ]
    const next = newHits(fightOf(log), first.mark, NOW)
    expect(next.hits.map((h) => (h.kind === 'hit' ? [h.target, h.amount, h.down] : null))).toEqual([
      ['Pillard', 4, false],
      ['Kaël', 3, true],
    ])
    // Nothing new, nothing played.
    const again = newHits(fightOf(log), next.mark, NOW)
    expect(again.hits).toEqual([])
    // A shorter log is a new fight, read from its start.
    const fresh = newHits(fightOf([damaged('pc_kael', 10, 8, 2)]), next.mark, NOW)
    expect(fresh.hits.map((h) => h.kind === 'hit' && h.amount)).toEqual([2])
  })
})
