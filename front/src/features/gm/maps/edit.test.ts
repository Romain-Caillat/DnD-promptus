import { describe, expect, it } from 'vitest'
import type { MapData } from '@/lib/board'
import { erase, paint, place, toggleDoor } from './edit'

const ROOM: MapData = {
  id: 'salle',
  name: 'Salle',
  theme: 'port-1718',
  ambience: {},
  grid: {
    legend: {
      '#': { terrain: 'pierre', wall: true },
      '.': { terrain: 'pavés' },
    },
    rows: ['#####', '#...#', '#####'],
  },
}

describe('map editing', () => {
  it('paints with one glyph per kind of cell and forgets the glyphs no cell uses', () => {
    const water = paint(
      ROOM,
      [
        [1, 1],
        [2, 1],
      ],
      { terrain: 'eau', water: 'deep' },
    )
    expect(water.grid.rows).toEqual(['#####', '#,,.#', '#####'])
    expect(water.grid.legend[',']).toEqual({ terrain: 'eau', water: 'deep' })
    const floor = paint(
      water,
      [
        [1, 1],
        [2, 1],
        [3, 1],
      ],
      { terrain: 'pierre', wall: true },
    )
    expect(floor.grid.rows).toEqual(['#####', '#####', '#####'])
    expect(Object.keys(floor.grid.legend)).toEqual(['#'])
  })

  it('walls a door’s cell, cycles its state, and drops it when its wall is painted over', () => {
    let map = toggleDoor(ROOM, [2, 1], 'base')
    expect(map.grid.rows[1]).toBe('#.,.#')
    expect(map.grid.legend[',']).toEqual({
      terrain: 'pavés',
      wall: true,
      void: false,
    })
    expect(map.doors).toEqual([{ id: 'porte-1', at: [2, 1], state: 'closed', layer: 'base' }])
    map = toggleDoor(map, [2, 1], 'base')
    expect(map.doors?.[0].state).toBe('open')
    map = toggleDoor(toggleDoor(map, [2, 1], 'base'), [2, 1], 'base')
    expect(map.doors).toEqual([])
    map = toggleDoor(map, [2, 1], 'base')
    expect(paint(map, [[2, 1]], { terrain: 'pavés' }).doors).toEqual([])
  })

  it('places things with ids of their own and erases all that stands on a cell', () => {
    let map = place(ROOM, [1, 1], { kind: 'object', label: 'Trappe' }, 'secrets')
    map = place(map, [1, 1], { kind: 'light' }, 'base')
    map = place(map, [3, 1], { kind: 'start', side: 'foes' }, 'base')
    map = place(map, [2, 1], { kind: 'prop', prop: 'caisses', label: 'Caisses' }, 'base')
    expect(map.objects?.[0]).toMatchObject({
      id: 'objet-1',
      label: 'Trappe',
      layer: 'secrets',
    })
    expect(map.starts?.[0]).toMatchObject({
      id: 'depart-1',
      side: 'foes',
      at: [3, 1],
    })
    const erased = erase(map, [1, 1])
    expect(erased.objects).toEqual([])
    expect(erased.lights).toEqual([])
    expect(erased.props).toHaveLength(1)
    expect(place(ROOM, [9, 9], { kind: 'light' }, 'base')).toBe(ROOM)
  })
})
