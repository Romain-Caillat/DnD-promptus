import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { LiveScreen } from '@/lib/evening'
import { EndPanel } from './EndPanel'

const LIVE = {
  session: { id: 's1', number: 1, status: 'live' },
  ai: { configured: true },
  gaps: [
    {
      node: 'sc_depart',
      nodeTitle: "Transition vers l'acte 2",
      revelation: 'rev_route',
      statement: 'Le Greyhound part par la passe du Nord.',
      clues: [{ clue: 'cl_registre', node: 'sc_bureau_morel', nodeTitle: 'Le bureau du capitaine de port' }],
    },
  ],
} as unknown as LiveScreen

describe('EndPanel', () => {
  it('shows what the table misses for what follows, and slips it into « Précédemment… »', async () => {
    const onEnd = vi.fn(async () => {})
    render(<EndPanel screen={LIVE} onEnd={onEnd} onDraft={async () => null} />)

    expect(screen.getByText("« Transition vers l'acte 2 » demande : Le Greyhound part par la passe du Nord.")).toBeInTheDocument()
    expect(screen.getByText('Encore à trouver dans : Le bureau du capitaine de port')).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText(/Précédemment/), 'Le quai a brûlé.')
    await userEvent.click(screen.getByRole('button', { name: /Ajouter au/ }))
    expect(screen.getByRole('button', { name: 'Ajouté' })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: 'Terminer la séance' }))
    expect(onEnd).toHaveBeenCalledWith('', 'Le quai a brûlé.\nÀ retenir : Le Greyhound part par la passe du Nord.')
  })
})
