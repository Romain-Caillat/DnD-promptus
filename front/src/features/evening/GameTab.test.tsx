import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo, stubReducedMotion } from '@/test-utils'
import { GameTab } from './GameTab'

const CAMPAIGN = { title: 'C', world: 'W', playerHook: '', clues: [], npcs: [] }
const SCENE = {
  id: 'sc_taverne',
  act: 'acte_1',
  title: 'Le Goéland Ivre',
  readAloud: 'La pluie bat les carreaux.',
  place: { name: 'La taverne', description: '' },
  npcs: [],
  opponents: [],
}
const ROLL = {
  die: '1d20',
  faces: [14],
  natural: 14,
  advantage: 'normal',
  modifiers: [{ source: { from: 'ability', id: 'DEX' }, value: 2 }],
  total: 16,
  target: { against: 'difficulty', id: 'moyen', value: 12 },
  band: 'success',
}
const request = (status: string, roll: unknown = null) => ({
  id: 'r1',
  card: { kind: 'ability', id: 'DEX', name: 'Dextérité' },
  text: 'Je subtilise la clé.',
  status,
  gmReason: null,
  check: { ability: 'DEX', abilityName: 'Dextérité', difficulty: 12, label: 'Moyen' },
  roll,
  outcome: roll ? 'Réussite' : null,
  contested: false,
  createdAt: '2026-10-06T20:00:00Z',
})
const evening = (over: Record<string, unknown>) => ({
  status: 200,
  body: {
    data: {
      session: { number: 2, status: 'live', startedAt: '2026-10-06T20:00:00Z' },
      previously: null,
      music: null,
      lobby: [],
      campaign: { ...CAMPAIGN, scene: SCENE },
      journal: [],
      requests: [],
      cards: [{ kind: 'ability', id: 'DEX', name: 'Dextérité', description: '', modifier: 2 }],
      feedback: null,
      ...over,
    },
  },
})
const MEDIA = { 'GET /api/play/c1/media': () => ({ status: 200, body: { data: { assets: [], theme: null } } }) }

describe('GameTab', () => {
  beforeEach(() => stubReducedMotion(false))
  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  it('arrives in the lobby with the sound checked', async () => {
    vi.stubGlobal(
      'AudioContext',
      class {
        currentTime = 0
        destination = {}
        createOscillator() {
          return { frequency: {}, connect() {}, start() {}, stop() {} }
        }
      },
    )
    const fetchMock = mockApi({
      ...MEDIA,
      'GET /api/play/c1/evening': () => evening({ session: { number: 2, status: 'lobby', startedAt: null } }),
      'POST /api/play/c1/lobby': () => evening({ session: { number: 2, status: 'lobby', startedAt: null } }),
    })
    render(<GameTab campaignId="c1" refreshKey={0} seated />)
    await userEvent.click(await screen.findByRole('button', { name: 'Tester le son' }))
    await userEvent.click(screen.getByRole('button', { name: /Je suis là/ }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/lobby')).toEqual([{ soundOk: true, remote: true }])
  })

  it('plays the act’s introduction above the scene once the GM kept it', async () => {
    mockApi({
      'GET /api/play/c1/media': () => ({
        status: 200,
        body: {
          data: {
            assets: [
              { id: 'v1', kind: 'intro', subject: 'acte_1' },
              { id: 'v0', kind: 'intro', subject: 'acte_2' },
            ],
            theme: null,
          },
        },
      }),
      'GET /api/play/c1/evening': () => evening({}),
    })
    render(<GameTab campaignId="c1" refreshKey={0} seated />)
    expect(await screen.findByLabelText("Introduction de l'acte")).toHaveAttribute('src', '/api/play/c1/media/v1/image')
  })

  it('asks with a card, then rolls the check: the die lands on the server’s face', async () => {
    const fetchMock = mockApi({
      ...MEDIA,
      'GET /api/play/c1/evening': () => evening({}),
      'POST /api/play/c1/requests': () => evening({ requests: [request('check')] }),
      'POST /api/play/c1/requests/r1/roll': () => evening({ requests: [request('rolled', ROLL)] }),
    })
    render(<GameTab campaignId="c1" refreshKey={0} seated />)
    expect(await screen.findByRole('heading', { name: 'Le Goéland Ivre' })).toBeInTheDocument()

    await userEvent.click(screen.getByRole('button', { name: /Dextérité/ }))
    await userEvent.type(screen.getByRole('textbox'), 'Je subtilise la clé.')
    await userEvent.click(screen.getByRole('button', { name: /Proposer au MJ/ }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/requests')).toEqual([
      { card: { kind: 'ability', ability: 'DEX' }, text: 'Je subtilise la clé.' },
    ])

    vi.useFakeTimers({ shouldAdvanceTime: true })
    await userEvent.click(await screen.findByRole('button', { name: /Lancer le dé/ }))
    const die = await screen.findByRole('img', { name: /Dé à 20 faces : 14/ })
    act(() => vi.advanceTimersByTime(1500))
    expect(die).toHaveAttribute('data-rolling', 'false')
    expect(screen.getByTestId('die-face')).toHaveTextContent('14')
    expect(screen.getAllByText('Réussite').length).toBeGreaterThan(0)
  })

  it('plays from the keyboard on a computer: a digit picks, Enter sends, Space rolls', async () => {
    const fetchMock = mockApi({
      ...MEDIA,
      'GET /api/play/c1/evening': () => evening({}),
      'POST /api/play/c1/requests': () => evening({ requests: [request('check')] }),
      'POST /api/play/c1/requests/r1/roll': () => evening({ requests: [request('rolled', ROLL)] }),
    })
    render(<GameTab campaignId="c1" refreshKey={0} seated keyboard />)
    expect(await screen.findByRole('heading', { name: 'Le Goéland Ivre' })).toBeInTheDocument()

    await userEvent.keyboard('1')
    expect(screen.getByRole('button', { pressed: true })).toHaveTextContent(/^Dextérité/)
    // The text box takes the focus: a space typed there is a space, not a roll.
    await userEvent.keyboard('Je file {Enter}')
    expect(sentTo(fetchMock, 'POST /api/play/c1/requests')).toEqual([
      { card: { kind: 'ability', ability: 'DEX' }, text: 'Je file' },
    ])
    expect(sentTo(fetchMock, 'POST /api/play/c1/requests/r1/roll')).toHaveLength(0)

    expect(await screen.findByText(/Espace marche aussi/)).toBeInTheDocument()
    ;(document.activeElement as HTMLElement | null)?.blur()
    await userEvent.keyboard(' ')
    expect(sentTo(fetchMock, 'POST /api/play/c1/requests/r1/roll')).toHaveLength(1)
  })

  it('keeps the keys off the hand on a phone', async () => {
    mockApi({ ...MEDIA, 'GET /api/play/c1/evening': () => evening({}) })
    render(<GameTab campaignId="c1" refreshKey={0} seated />)
    expect(await screen.findByRole('heading', { name: 'Le Goéland Ivre' })).toBeInTheDocument()
    await userEvent.keyboard('1')
    expect(screen.queryByRole('button', { pressed: true })).not.toBeInTheDocument()
  })

  it('answers the three questions after the session', async () => {
    const fetchMock = mockApi({
      ...MEDIA,
      'GET /api/play/c1/evening': () => evening({ session: null, feedback: { number: 1, answered: false } }),
      'POST /api/play/c1/feedback': () => evening({ session: null, feedback: { number: 1, answered: true } }),
    })
    render(<GameTab campaignId="c1" refreshKey={0} seated />)
    for (const q of ['Les règles étaient-elles claires ?', 'As-tu eu ton moment ?', 'Sais-tu quoi faire la prochaine fois ?']) {
      const group = (await screen.findByText(q)).closest('fieldset')!
      await userEvent.click(group.querySelector('button')!)
    }
    await userEvent.click(screen.getByRole('button', { name: 'Envoyer' }))
    expect(sentTo(fetchMock, 'POST /api/play/c1/feedback')).toEqual([
      { rulesClear: 'yes', hadMoment: 'yes', knowsNext: 'yes', comment: '' },
    ])
    expect(await screen.findByText('Merci, le MJ a ta réponse.')).toBeInTheDocument()
  })
})
