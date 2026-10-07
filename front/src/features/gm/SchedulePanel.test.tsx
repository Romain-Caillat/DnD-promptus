import { fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { SchedulePanel } from './SchedulePanel'

const SCHEDULE = {
  number: 4,
  dates: [
    {
      id: 'd1',
      startsAt: '2030-10-10T18:30:00Z',
      minutes: 150,
      status: 'proposed',
      lobbyOpensAt: '2030-10-10T18:15:00Z',
      answers: [
        { playerId: 'p1', nickname: 'Marc', available: true },
        { playerId: 'p2', nickname: 'Camille', available: false },
      ],
      log: [],
    },
  ],
  players: [
    { id: 'p1', nickname: 'Marc' },
    { id: 'p2', nickname: 'Camille' },
    { id: 'p3', nickname: 'Hugo' },
  ],
  discord: { configured: false, hint: null },
  lobbyBeforeMinutes: 15,
}

describe('SchedulePanel', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('shows who can make each date, proposes another, fixes one and plugs the Discord channel', async () => {
    const fetchMock = mockApi({
      'GET /api/campaigns/c1/schedule': () => ({ status: 200, body: { data: SCHEDULE } }),
      'POST /api/campaigns/c1/schedule/dates': () => ({ status: 201, body: { data: {} } }),
      'POST /api/campaigns/c1/schedule/dates/d1/choose': () => ({ status: 200, body: { data: {} } }),
      'PUT /api/campaigns/c1/schedule/discord': () => ({ status: 204 }),
    })
    render(<SchedulePanel campaignId="c1" refreshKey={0} />)

    const proposed = await screen.findByRole('region', { name: 'Dates proposées à la table' })
    expect(within(proposed).getByText('1 sur 3 peuvent')).toBeInTheDocument()
    const answers = within(proposed).getByRole('list', { name: 'Réponses' })
    expect(within(answers).getByText('Camille · ne peut pas')).toBeInTheDocument()
    expect(within(answers).getByText('Hugo · pas encore répondu')).toBeInTheDocument()

    fireEvent.change(within(proposed).getByLabelText('Date et heure'), { target: { value: '2030-10-11T20:30' } })
    await userEvent.click(within(proposed).getByRole('button', { name: 'Proposer cette date' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/schedule/dates')).toEqual([
      { startsAt: new Date('2030-10-11T20:30').toISOString(), minutes: 150 },
    ])

    await userEvent.click(within(proposed).getByRole('button', { name: 'Fixer cette date' }))
    expect(sentTo(fetchMock, 'POST /api/campaigns/c1/schedule/dates/d1/choose')).toHaveLength(1)

    const discord = screen.getByRole('region', { name: 'Le salon Discord de la table' })
    await userEvent.type(within(discord).getByRole('textbox'), 'https://discord.com/api/webhooks/1/abc')
    await userEvent.click(within(discord).getByRole('button', { name: 'Brancher' }))
    expect(sentTo(fetchMock, 'PUT /api/campaigns/c1/schedule/discord')).toEqual([
      { webhook: 'https://discord.com/api/webhooks/1/abc' },
    ])
  })

  it('says why the server refused a date', async () => {
    mockApi({
      'GET /api/campaigns/c1/schedule': () => ({ status: 200, body: { data: { ...SCHEDULE, dates: [] } } }),
      'POST /api/campaigns/c1/schedule/dates': () => ({
        status: 400,
        body: { error: { code: 'DATE_IN_PAST', message: 'past' } },
      }),
    })
    render(<SchedulePanel campaignId="c1" refreshKey={0} />)
    await userEvent.click(await screen.findByRole('button', { name: 'Proposer cette date' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Cette date est déjà passée.')
  })
})
