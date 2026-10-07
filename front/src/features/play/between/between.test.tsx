import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { BetweenView } from '@/lib/between'
import type { ActionCardView, CharacterView, PlayView } from '@/lib/play'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { CardDetail } from './CardDetail'
import { Chronicle } from './Chronicle'
import { EveningEnd } from './EveningEnd'
import { LevelUpPanel } from './LevelUpPanel'

const CARD: ActionCardView = {
  id: 'fente',
  name: 'Fente',
  description: 'Une attaque qui traverse la garde.',
  kind: 'Attaque',
  level: 3,
  attackBonus: 4,
  damage: '3',
  heal: null,
  cooldown: 2,
  range: 1,
}

const BETWEEN: BetweenView = {
  open: null,
  last: {
    number: 3,
    endedAt: '2026-10-03T23:10:00Z',
    minutes: 160,
    title: 'La crypte des Valombre',
    previously: 'Sous l’autel, Borin a trouvé la page arrachée.',
    learnt: ['La page arrachée nomme la porte nord.'],
    mine: {
      xpGained: 6,
      levelBefore: 2,
      level: 3,
      totalXp: 11,
      xpBar: 1,
      xpBarMax: 5,
      nextLevelXp: 15,
      got: ['Hache des Valombre — Borin'],
    },
  },
  levelUp: {
    level: 3,
    points: 1,
    abilities: [{ id: 'FOR', name: 'Force', score: 14, modifier: 2 }],
    newCards: [CARD],
  },
  chronicle: [
    { number: 1, endedAt: '2026-09-19T12:00:00Z', title: 'Une ville qui enterre deux fois', text: 'Le maire ment.' },
    { number: 2, endedAt: null, title: null, text: null },
  ],
  openThreads: ['Ramener la lampe de Dorn.'],
}

const PLAY: PlayView = {
  level: 3,
  totalXp: 11,
  xpBar: 1,
  xpBarMax: 5,
  upgradePoints: 1,
  nextLevelXp: 15,
  hitPoints: 10,
  maxHitPoints: 10,
  armorClass: 12,
  initiative: 2,
  abilities: [
    { id: 'FOR', name: 'Force', score: 14, modifier: 2 },
    { id: 'CHA', name: 'Charisme', score: 9, modifier: -1 },
  ],
  cards: [CARD],
  resources: [],
  inventory: [],
}

const CHARACTER: CharacterView = {
  id: 'ch1',
  status: 'validated',
  sheet: { name: 'Borin', classId: 'bretteur' },
  gmNote: null,
  updatedAt: '2026-10-03T23:10:00Z',
  peopleName: null,
  className: 'Bretteur',
  stats: null,
  play: PLAY,
}

beforeEach(() => stubReducedMotion(true))
afterEach(() => vi.unstubAllGlobals())

describe('EveningEnd', () => {
  it('tells the evening, the level reached, what Borin got, then leads to the level-up', async () => {
    const onLevelUp = vi.fn()
    render(<EveningEnd between={BETWEEN} onLevelUp={onLevelUp} onChronicle={() => {}} onSheet={() => {}} />)
    expect(screen.getByText(/Fin de la séance 3 · 2 h 40 de jeu/)).toBeInTheDocument()
    expect(screen.getByText(/\+6 XP ce soir · niveau 3 atteint !/)).toBeInTheDocument()
    expect(screen.getByText('Hache des Valombre — Borin')).toBeInTheDocument()
    expect(screen.getByText('Sous l’autel, Borin a trouvé la page arrachée.')).toBeInTheDocument()
    expect(screen.getByText('La page arrachée nomme la porte nord.')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Passer au niveau 3/ }))
    expect(onLevelUp).toHaveBeenCalled()
  })

  it('waits for the GM’s recap before telling the story, and offers points without a new level', () => {
    const between: BetweenView = {
      ...BETWEEN,
      last: { ...BETWEEN.last!, previously: null, mine: { ...BETWEEN.last!.mine!, levelBefore: 3 } },
    }
    render(<EveningEnd between={between} onLevelUp={() => {}} onChronicle={() => {}} onSheet={() => {}} />)
    expect(screen.getByText(/relit le récapitulatif/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Placer ton point d'amélioration/ })).toBeInTheDocument()
  })

  it('shows a spectator the table’s evening, nothing of a character', () => {
    const between: BetweenView = { ...BETWEEN, levelUp: null, last: { ...BETWEEN.last!, mine: null } }
    render(<EveningEnd between={between} onLevelUp={() => {}} onChronicle={() => {}} onSheet={() => {}} />)
    expect(screen.queryByText(/XP ce soir/)).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Voir ta fiche/ })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Lire la chronique/ })).toBeInTheDocument()
  })
})

describe('LevelUpPanel', () => {
  it('spends a point on the chosen ability and shows the card the level opened', async () => {
    const raised: CharacterView = {
      ...CHARACTER,
      play: { ...PLAY, upgradePoints: 0, abilities: [{ id: 'FOR', name: 'Force', score: 15, modifier: 2 }, PLAY.abilities[1]] },
    }
    const fetchMock = mockApi({
      'GET /api/play/c1/between': () => ({ status: 200, body: { data: BETWEEN } }),
      'POST /api/play/c1/character/upgrade': () => ({ status: 200, body: { data: raised } }),
    })
    const onChanged = vi.fn()
    render(<LevelUpPanel campaignId="c1" character={CHARACTER} play={PLAY} refreshKey={0} onChanged={onChanged} />)
    expect(await screen.findByText('Fente')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('radio', { name: /Force/ }))
    await userEvent.click(screen.getByRole('button', { name: /Ajouter 1 en Force/ }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/character/upgrade')).toEqual([{ ability: 'FOR' }])
    await waitFor(() => expect(onChanged).toHaveBeenCalledWith(raised))
  })

  it('keeps the numbers still while a session is live', async () => {
    mockApi({
      'GET /api/play/c1/between': () => ({
        status: 200,
        body: { data: { ...BETWEEN, open: { number: 4, status: 'live', startedAt: null } } },
      }),
    })
    render(<LevelUpPanel campaignId="c1" character={CHARACTER} play={PLAY} refreshKey={0} onChanged={() => {}} />)
    expect(await screen.findByText(/en pleine partie/)).toBeInTheDocument()
    expect(screen.queryByRole('radio')).not.toBeInTheDocument()
  })

  it('says why the server refused', async () => {
    mockApi({
      'GET /api/play/c1/between': () => ({ status: 200, body: { data: BETWEEN } }),
      'POST /api/play/c1/character/upgrade': () => ({
        status: 409,
        body: { error: { code: 'NO_UPGRADE_POINT', message: '' } },
      }),
    })
    render(<LevelUpPanel campaignId="c1" character={CHARACTER} play={PLAY} refreshKey={0} onChanged={() => {}} />)
    await userEvent.click(await screen.findByRole('radio', { name: /Charisme/ }))
    await userEvent.click(screen.getByRole('button', { name: /Ajouter 1 en Charisme/ }))
    expect(await screen.findByRole('alert')).toHaveTextContent('plus de point')
  })
})

describe('Chronicle', () => {
  it('lists every session played, the last marked, and what stays open', async () => {
    mockApi({ 'GET /api/play/c1/between': () => ({ status: 200, body: { data: BETWEEN } }) })
    render(<Chronicle campaignId="c1" refreshKey={0} />)
    expect(await screen.findByText('Une ville qui enterre deux fois')).toBeInTheDocument()
    expect(screen.getByText(/Séance 1 · 19 sept/)).toBeInTheDocument()
    expect(screen.getByText('Le MJ n’a rien publié pour cette séance.'.replace('’', "'"))).toBeInTheDocument()
    expect(screen.getByText('Ramener la lampe de Dorn.')).toBeInTheDocument()
  })
})

describe('CardDetail', () => {
  it('reads what the card does with the server’s numbers', () => {
    render(<CardDetail card={CARD} onClose={() => {}} />)
    expect(screen.getByText('Attaque · dès le niveau 3')).toBeInTheDocument()
    expect(screen.getByText('Toucher : +4 au dé')).toBeInTheDocument()
    expect(screen.getByText('Recharge : 2 tours')).toBeInTheDocument()
  })
})
