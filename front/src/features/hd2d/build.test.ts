import { describe, expect, it } from 'vitest'
import type { MapData } from '@/lib/board'
import type { Needs } from '@/lib/elements'
import type { Asset, Tileset } from '@/lib/media'
import { batchOf } from '@/features/gm/atlas/claude'
import { readKey } from '@/features/gm/atlas/cast'
import { buildScene, type BuildInput } from './build'

const TILESET: Tileset = {
  id: 'port-1718',
  name: 'Le port',
  void: '#07090c',
  walls: { cap: '#8c8577', face: '#5b5448', joint: '#2f2b25', pattern: 'stones' },
  materials: {
    pavés: { name: 'Pavés', base: '#5d5850', dark: '#3c3934', light: '#7a746a', pattern: 'cobbles' },
    eau: { name: 'Eau', base: '#173040', dark: '#0d1e2a', light: '#2c5268', pattern: 'water' },
  },
  props: {
    caisse: { name: 'Caisse', shape: 'crate', color: '#8a6239', accent: '#4f3720' },
    cordages: { name: 'Cordages', shape: 'rope', color: '#b59a62', accent: '#7a6438' },
  },
}

const MAP: MapData = {
  id: 'quai',
  name: 'Le quai',
  theme: 'port-1718',
  ambience: { time: 'night', weather: 'fog' },
  grid: {
    legend: { '.': { terrain: 'pavés' }, '#': { terrain: 'pierre', wall: true }, '~': { terrain: 'eau', water: 'deep' } },
    rows: ['#####', '#...#', '~~~~~'],
  },
  props: [
    { id: 'c1', kind: 'caisse', at: [1, 1] },
    { id: 'r1', kind: 'cordages', at: [3, 1] },
  ],
  lights: [{ id: 'l1', at: [2, 1], bright: 2, dim: 4, flicker: true }],
}

const NEEDS: Needs = {
  tileset: 'port-1718',
  palette: [],
  generations: 4,
  needs: [
    { kind: 'element', subject: 'port-1718/material/pavés', name: 'Sol · Pavés', form: 'texture', status: 'approved', asset: 'a-paves', generations: 0 },
    { kind: 'element', subject: 'port-1718/material/eau', name: 'Sol · Eau', form: 'texture', status: 'missing', asset: null, generations: 1 },
    { kind: 'element', subject: 'port-1718/wall', name: 'Murs', form: 'texture', status: 'pending', asset: 'a-mur', generations: 0 },
    { kind: 'element', subject: 'port-1718/prop/caisse/side', name: 'Caisse · côté', form: 'texture', status: 'missing', asset: null, generations: 1 },
    { kind: 'element', subject: 'port-1718/prop/caisse/top', name: 'Caisse · dessus', form: 'texture', status: 'missing', asset: null, generations: 1 },
    { kind: 'element', subject: 'port-1718/prop/cordages', name: 'Cordages', form: 'flat', status: 'missing', asset: null, generations: 1 },
  ],
}

const ASSETS: Asset[] = [{ id: 'f-gwen', kind: 'character', subject: 'pnj_gwen', status: 'pending', meta: { name: 'Gwen' } }]

function input(patch: Partial<BuildInput> = {}): BuildInput {
  return {
    map: MAP,
    tileset: TILESET,
    needs: NEEDS,
    assets: ASSETS,
    marks: [
      { key: 'npc:pnj_gwen:0', at: [2, 1], label: 'G', title: 'Gwen', shape: 'npc' },
      { key: 'claude:p1|npc:pnj_loic:0', at: [3, 1], label: 'L', title: 'Loïc', shape: 'npc', proposed: true },
      { key: 'lieu_quai', at: [1, 1], label: '1', title: 'Le quai' },
    ],
    proposedCells: [[4, 1]],
    imageUrl: (id) => `/img/${id}`,
    npcOf: (key) => {
      const k = readKey(key)
      return k?.kind === 'npc' ? k.id : null
    },
    batchOf,
    ...patch,
  }
}

describe('buildScene (campaign/watch-prep-in-hd2d)', () => {
  it('lays the floors, walls and water from the grid, with PixelLab textures where they exist', () => {
    const s = buildScene(input())
    expect([s.width, s.height]).toEqual([5, 3])
    expect(s.walls).toHaveLength(7)
    const paves = s.floors.find((f) => f.material === 'pavés')!
    expect(paves.cells).toEqual([[1, 1], [2, 1], [3, 1]])
    expect(paves.paint).toEqual({ kind: 'image', url: '/img/a-paves', pending: false, subject: 'port-1718/material/pavés' })
    const water = s.floors.find((f) => f.material === 'eau')!
    expect(water.water).toBe('deep')
    // Missing: drawn from the theme's colours, never hidden.
    expect(water.paint).toMatchObject({ kind: 'theme', base: '#173040', pattern: 'water' })
    // The walls' element waits for the GM: drawn, marked pending.
    expect(s.wallPaint).toMatchObject({ kind: 'image', pending: true })
    expect([s.time, s.fog]).toEqual(['night', true])
    expect(s.lights).toEqual([{ at: [2, 1], color: '#ffb060', bright: 2, flicker: true }])
  })

  it('makes a crate a volume and a rope a flat image, by the shape rule', () => {
    const s = buildScene(input())
    expect(s.volumes.map((v) => v.id)).toEqual(['c1'])
    expect(s.volumes[0]!.side).toMatchObject({ kind: 'theme', base: '#8a6239' })
    expect(s.flats.map((f) => f.id)).toEqual(['r1'])
    expect(s.flats[0]!.paint).toBeNull()
  })

  it("stands the marks up, Claude's apart, and an NPC with its PixelLab figure", () => {
    const s = buildScene(input())
    const gwen = s.figures.find((f) => f.key === 'npc:pnj_gwen:0')!
    expect(gwen).toMatchObject({ shape: 'npc', proposal: null, image: { url: '/img/f-gwen', pending: true } })
    const loic = s.figures.find((f) => f.title === 'Loïc')!
    expect(loic).toMatchObject({ proposal: 'p1', image: null })
    expect(s.figures.find((f) => f.key === 'lieu_quai')!.shape).toBe('pin')
    expect(s.proposedCells).toEqual([[4, 1]])
  })

  it('lists what is missing and what waits for the GM', () => {
    const s = buildScene(input())
    expect(s.missing.map((m) => m.subject)).toEqual([
      'port-1718/material/eau',
      'port-1718/prop/caisse/side',
      'port-1718/prop/caisse/top',
      'port-1718/prop/cordages',
      // Loïc, proposed by Claude, has no figure yet.
      'pnj_loic',
    ])
    expect(s.pending).toEqual([
      { asset: 'a-mur', subject: 'port-1718/wall', name: 'Murs' },
      { asset: 'f-gwen', subject: 'pnj_gwen', name: 'Gwen' },
    ])
  })

  it('still draws without the needs (loading): every prop a volume, nothing listed', () => {
    const s = buildScene(input({ needs: null }))
    expect(s.volumes.map((v) => v.id)).toEqual(['c1', 'r1'])
    expect(s.floors.every((f) => f.paint.kind === 'theme')).toBe(true)
    expect(s.missing.map((m) => m.subject)).toEqual(['pnj_loic'])
  })
})
