import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { GmBoard } from '@/lib/board'
import { stubReducedMotion } from '@/test-utils'
import { BoardPanel } from './BoardPanel'

const MAP = {
  id: 'crypte',
  name: 'La crypte',
  theme: 'port-1718',
  ambience: {},
  grid: { legend: { '.': { terrain: 'dalles' } }, rows: ['....', '....'] },
}

const combatant = (id: string, name: string, side: 'party' | 'opposition', hp: number, saves: unknown = null) => ({
  id,
  name,
  side,
  hit_points: hp,
  conditions: [],
  death_saves: saves,
})

function data(borinSaves: unknown): GmBoard {
  return {
    board: {
      mapId: 'crypte',
      map: MAP,
      fog: false,
      revealed: [],
      tokens: [],
    },
    maps: [],
    encounters: [],
    conditions: null,
    encounter: {
      id: 'e1',
      node: 'crypte',
      live: true,
      version: 3,
      fight: {
        scene: {
          combatants: {
            'pc-b': combatant('pc-b', 'Borin', 'party', 0, borinSaves),
            'pc-s': combatant('pc-s', 'Sef', 'party', 0),
            chef: combatant('chef', 'Chef gobelin', 'opposition', 18),
          },
        },
        positions: {},
        order: ['chef', 'pc-b', 'pc-s'],
        standing: { chef: 'in_fight', 'pc-b': 'in_fight', 'pc-s': 'in_fight' },
        round: 4,
        turn: 0,
        end: null,
      },
      maxHitPoints: { 'pc-b': 31, 'pc-s': 24, chef: 27 },
      proposal: null,
      loot: [],
      events: [],
      reachable: [],
    },
  } as unknown as GmBoard
}

function renderPanel(d: GmBoard) {
  const onCommand = vi.fn()
  render(
    <BoardPanel
      campaignId="c1"
      data={d}
      media={null}
      live
      onShow={() => {}}
      onEdit={() => {}}
      onStart={() => {}}
      onCommand={onCommand}
      onLoot={() => {}}
    />,
  )
  return onCommand
}

describe('BoardPanel, a death in the fight', () => {
  beforeEach(() => stubReducedMotion(true))
  afterEach(() => vi.unstubAllGlobals())

  it('lets the GM confirm the death the dice propose, or decide another outcome', async () => {
    const onCommand = renderPanel(data({ successes: 1, failures: 3, stable: false, death_due: true }))
    expect(screen.getByText(/contre la mort : 1 ✓ · 3 ✗/)).toBeInTheDocument()
    expect(screen.getByRole('alert')).toHaveTextContent('Les dés proposent la mort de Borin.')
    await userEvent.click(screen.getByRole('button', { name: 'Autre issue : stabilisé' }))
    await userEvent.click(screen.getByRole('button', { name: 'Confirmer : Borin meurt' }))
    expect(onCommand.mock.calls.map((c) => c[0])).toEqual([
      { kind: 'spare', who: 'pc-b' },
      { kind: 'confirmDeath', who: 'pc-b' },
    ])
  })

  it('lets the GM decide a death for a character down at 0, in two steps', async () => {
    const onCommand = renderPanel(data(null))
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Sef meurt…' }))
    expect(onCommand).not.toHaveBeenCalled()
    await userEvent.click(screen.getByRole('button', { name: 'Confirmer : Sef meurt' }))
    expect(onCommand).toHaveBeenCalledWith({ kind: 'confirmDeath', who: 'pc-s' })
  })
})

describe('BoardPanel, the scenes pinned on the map', () => {
  beforeEach(() => stubReducedMotion(true))
  afterEach(() => vi.unstubAllGlobals())

  const pinned = {
    board: { mapId: 'crypte', map: MAP, fog: false, revealed: [], tokens: [] },
    pins: [
      { location: 'l_porte', name: 'La porte', at: [0, 0], scenes: [{ id: 'sc_porte', title: 'La porte scellée', number: 4 }] },
      {
        location: 'l_ossuaire',
        name: 'L’ossuaire',
        at: [3, 1],
        scenes: [
          { id: 'sc_os', title: 'Les os qui parlent', number: 5 },
          { id: 'sc_fuite', title: 'La fuite', number: 7 },
        ],
      },
    ],
    maps: [],
    encounters: [],
    conditions: null,
    encounter: null,
  } as unknown as GmBoard

  it('launches the scene of a pin touched, and lets the GM pick when several share it', async () => {
    const onScene = vi.fn()
    render(
      <BoardPanel
        campaignId="c1"
        data={pinned}
        media={null}
        live
        onShow={() => {}}
        onEdit={() => {}}
        onStart={() => {}}
        onCommand={() => {}}
        onLoot={() => {}}
        onScene={onScene}
      />,
    )
    await userEvent.click(screen.getByRole('button', { name: 'La porte — lancer : 4. La porte scellée' }))
    expect(onScene).toHaveBeenLastCalledWith('sc_porte')

    const ossuary = screen.getByRole('button', { name: /L’ossuaire — lancer/ })
    expect(ossuary.textContent).toBe('5·7')
    await userEvent.click(ossuary)
    expect(onScene).toHaveBeenCalledTimes(1)
    await userEvent.click(screen.getByRole('button', { name: 'Lancer 7. La fuite' }))
    expect(onScene).toHaveBeenLastCalledWith('sc_fuite')
  })
})
