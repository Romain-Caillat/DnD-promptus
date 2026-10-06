import { describe, expect, it } from 'vitest'
import { pathTo } from '@/lib/board'
import type { MapData } from '@/lib/board'
import { E, N, S, W, cellAt, edgeMask, noise, readGrid } from './render'

const map: MapData = {
  id: 'm',
  name: 'M',
  theme: 't',
  ambience: {},
  grid: {
    legend: { '#': { terrain: 'pierre', wall: true }, '.': { terrain: 'pavés' }, '~': { terrain: 'eau' } },
    rows: ['#####', '#...#', '#.~.#', '#...#', '#####'],
  },
}

describe('the grid renderer', () => {
  it('picks one of sixteen autotile variants from the same-material sides', () => {
    const grid = readGrid(map)
    expect(edgeMask(grid, 2, 2)).toBe(0)
    expect(edgeMask(grid, 1, 1)).toBe(E | S)
    expect(edgeMask(grid, 2, 1)).toBe(E | W)
    expect(edgeMask(grid, 0, 2)).toBe(N | S)
    expect(edgeMask(grid, 2, 0)).toBe(E | W)
    for (let y = 0; y < 5; y++) for (let x = 0; x < 5; x++) expect(edgeMask(grid, x, y)).toBeLessThan(16)
  })

  it('reads accented glyphs as one cell', () => {
    const grid = readGrid({ ...map, grid: { legend: { é: { terrain: 'x' } }, rows: ['éé'] } })
    expect(grid.width).toBe(2)
    expect(grid.kind(1, 0)?.terrain).toBe('x')
  })

  it('answers the cell under a tap', () => {
    expect(cellAt(0, 0)).toEqual([0, 0])
    expect(cellAt(65, 33)).toEqual([2, 1])
  })

  it('draws the same noise on every screen', () => {
    expect(noise(3, 4, 1)).toBe(noise(3, 4, 1))
    expect(noise(3, 4, 1)).toBeGreaterThanOrEqual(0)
    expect(noise(3, 4, 1)).toBeLessThan(1)
  })

  it('walks a path through the reachable cells only', () => {
    const reach = [
      { at: [1, 0] as [number, number], cost: 1 },
      { at: [2, 1] as [number, number], cost: 2 },
      { at: [3, 1] as [number, number], cost: 3 },
    ]
    expect(pathTo([0, 0], [3, 1], reach)).toEqual([
      [1, 0],
      [2, 1],
      [3, 1],
    ])
    expect(pathTo([0, 0], [5, 5], reach)).toBeNull()
  })
})
