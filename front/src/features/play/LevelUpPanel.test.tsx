import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { PlayView } from '@/lib/play'
import { mockApi, sentTo } from '@/test-utils'
import { LevelUpPanel } from './LevelUpPanel'

const CARD = { kind: 'Attaque', attackBonus: 3, damage: '2', heal: null, cooldown: 2, range: 1, description: '' }

const borin = (over: Partial<PlayView> = {}): PlayView => ({
  level: 4,
  totalXp: 15,
  xpBar: 0,
  xpBarMax: 5,
  upgradePoints: 0,
  nextLevelXp: 20,
  hitPoints: 37,
  maxHitPoints: 37,
  armorClass: 12,
  initiative: 2,
  abilities: [
    { id: 'FOR', name: 'Force', score: 13, modifier: 1 },
    { id: 'CON', name: 'Constitution', score: 16, modifier: 3 },
  ],
  cards: [
    { ...CARD, id: 'estocade', name: 'Estocade', level: 1 },
    { ...CARD, id: 'riposte', name: 'Riposte en quarte', level: 4 },
  ],
  resources: [],
  inventory: [],
  levelHitPoints: { dice: '1d10', average: 6, bonus: 3, bonusFormula: 'mod(CON)' },
  levelsToChoose: [4],
  ...over,
})

describe('LevelUpPanel', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('offers the die or the average, and shows the card the level unlocks', async () => {
    const api = mockApi({
      'POST /api/play/c1/character/level-up': () => ({
        status: 200,
        body: {
          data: {
            taken: { level: 4, die: 7, faces: [7], maxBefore: 37, maxAfter: 38 },
            character: { play: borin({ levelsToChoose: [], maxHitPoints: 38 }) },
          },
        },
      }),
    })
    const changed = vi.fn()
    render(<LevelUpPanel campaignId="c1" play={borin()} onChanged={changed} />)
    expect(screen.getByRole('heading', { name: 'Niveau 4 !' })).toBeInTheDocument()
    // The average already counts the bonus: 6 + 3.
    expect(screen.getByRole('button', { name: /La moyenne/ })).toHaveTextContent('+9')
    expect(screen.getByRole('button', { name: /Lancer le dé/ })).toHaveTextContent('1d10 +3')
    expect(screen.getByText('Riposte en quarte')).toBeInTheDocument()
    expect(screen.queryByText('Estocade')).not.toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: /Lancer le dé/ }))
    expect(sentTo(api, 'POST /api/play/c1/character/level-up')).toEqual([{ level: 4, choice: 'roll' }])
    expect(await screen.findByRole('status')).toHaveTextContent('Le dé fait 7 : +1 PV, tu en as 38 au maximum.')
    expect(changed).toHaveBeenCalledOnce()
  })

  it('spends an upgrade point on the ability tapped', async () => {
    const api = mockApi({
      'POST /api/play/c1/character/upgrade': () => ({ status: 200, body: { data: { play: borin() } } }),
    })
    render(<LevelUpPanel campaignId="c1" play={borin({ levelsToChoose: [], upgradePoints: 2 })} onChanged={() => {}} />)
    expect(screen.getByText("2 points d'amélioration à placer")).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: '+1 en Constitution' }))
    expect(sentTo(api, 'POST /api/play/c1/character/upgrade')).toEqual([{ ability: 'CON' }])
  })

  it('shows nothing when there is nothing to choose', () => {
    const { container } = render(
      <LevelUpPanel campaignId="c1" play={borin({ levelsToChoose: [], levelHitPoints: null })} onChanged={() => {}} />,
    )
    expect(container).toBeEmptyDOMElement()
  })
})
