import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { GmBoard } from '@/lib/board'
import { DeathDecisions } from './DeathDecisions'

const enc = (dying: Record<string, unknown>) =>
  ({
    fight: {
      scene: { combatants: { 'pc-1': { name: 'Borin' }, 'pc-2': { name: 'Lyra' } } },
      dying,
    },
  }) as unknown as NonNullable<GmBoard['encounter']>

describe('DeathDecisions', () => {
  it('waits for the GM on a death the engine proposes', async () => {
    const onCommand = vi.fn()
    render(
      <DeathDecisions
        enc={enc({
          'pc-1': { successes: 0, failures: 3, stable: false, proposed: true },
          'pc-2': { successes: 1, failures: 1, stable: false, proposed: false },
        })}
        onCommand={onCommand}
      />,
    )
    expect(screen.getByText(/Trois échecs pour Borin/)).toBeInTheDocument()
    expect(screen.queryByText(/Lyra/)).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Confirmer : Borin meurt' }))
    await userEvent.click(screen.getByRole('button', { name: /Une autre issue/ }))
    expect(onCommand.mock.calls).toEqual([
      [{ kind: 'death', who: 'pc-1', call: 'die' }],
      [{ kind: 'death', who: 'pc-1', call: 'spare' }],
    ])
  })

  it('shows nothing when no death waits', () => {
    const { container } = render(<DeathDecisions enc={enc({})} onCommand={vi.fn()} />)
    expect(container).toBeEmptyDOMElement()
  })
})
