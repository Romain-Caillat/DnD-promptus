import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Plan } from '@/lib/between'
import type { LiveScreen, SessionInfo } from '@/lib/evening'
import { PlanPanel, RecapPanel } from './BetweenPanels'

const ENDED = {
  id: 's2',
  number: 2,
  status: 'ended',
  recap: 'Durée : 180 min.',
  previously: 'Précédemment : votre route vous a menés au quai.',
  title: 'Le quai',
  chronicle: '',
  published: false,
} as unknown as SessionInfo

describe('RecapPanel', () => {
  it('publishes what the GM reread, never before', async () => {
    const onSave = vi.fn(async () => {})
    const onPublish = vi.fn(async () => {})
    render(<RecapPanel session={ENDED} aiConfigured onSave={onSave} onPublish={onPublish} onDraft={async () => null} />)
    expect(screen.getByText('Pas encore publié')).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText(/Deux lignes/), 'Le feu prend.')
    await userEvent.click(screen.getByRole('button', { name: 'Publier aux joueurs' }))
    expect(onPublish).toHaveBeenCalledWith({
      recap: 'Durée : 180 min.',
      previously: 'Précédemment : votre route vous a menés au quai.',
      title: 'Le quai',
      chronicle: 'Le feu prend.',
    })
    expect(onSave).not.toHaveBeenCalled()
  })

  it('cannot publish an empty « Précédemment… », and takes the co-GM draft', async () => {
    const draft = { recap: 'r', previously: 'p', title: 't', chronicle: 'c' }
    render(
      <RecapPanel
        session={{ ...ENDED, previously: '' }}
        aiConfigured
        onSave={async () => {}}
        onPublish={async () => {}}
        onDraft={async () => draft}
      />,
    )
    expect(screen.getByRole('button', { name: 'Publier aux joueurs' })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: 'Brouillon du co-MJ' }))
    expect(screen.getByLabelText('Titre dans la chronique')).toHaveValue('t')
    expect(screen.getByRole('button', { name: 'Publier aux joueurs' })).toBeEnabled()
  })
})

const A = '2099-10-10T18:30:00Z'
const B = '2099-10-17T18:30:00Z'
const SCREEN = {
  lobby: [
    { playerId: 'p1', nickname: 'Léa', characterName: 'Borin', role: 'player' },
    { playerId: 'p2', nickname: 'Marc', characterName: '', role: 'player' },
    { playerId: 'p3', nickname: 'Zoé', characterName: '', role: 'spectator' },
  ],
} as unknown as LiveScreen

describe('PlanPanel', () => {
  it('shows who can on each date and fixes one', async () => {
    const plan: Plan = {
      options: [A, B],
      chosenAt: null,
      lobbyMinutes: 15,
      answers: [
        { playerId: 'p1', available: [A, B] },
        { playerId: 'p2', available: [B] },
      ],
    }
    const onChoose = vi.fn(async () => {})
    render(<PlanPanel plan={plan} screen={SCREEN} onPropose={async () => {}} onChoose={onChoose} />)
    expect(screen.getByText(/1 joueur sur 2 peut · Borin/)).toBeInTheDocument()
    expect(screen.getByText(/2 joueurs sur 2 peuvent · Borin, Marc/)).toBeInTheDocument()
    await userEvent.click(screen.getAllByRole('button', { name: 'Fixer cette date' })[1]!)
    expect(onChoose).toHaveBeenCalledWith(B)
  })

  it('proposes the dates typed, as instants', async () => {
    const onPropose = vi.fn(async () => {})
    render(<PlanPanel plan={null} screen={SCREEN} onPropose={onPropose} onChoose={async () => {}} />)
    const send = screen.getByRole('button', { name: 'Envoyer aux joueurs' })
    expect(send).toBeDisabled()
    await userEvent.type(screen.getByLabelText('Date 1'), '2099-10-10T20:30')
    await userEvent.click(send)
    expect(onPropose).toHaveBeenCalledWith([new Date('2099-10-10T20:30').toISOString()], 15)
  })
})
