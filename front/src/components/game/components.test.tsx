import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { stubReducedMotion } from '@/test-utils'
import { ArcadeCluster } from './ArcadeCluster'
import { BottomPanel } from './BottomPanel'
import { CardButton } from './CardButton'
import { ConditionBadge } from './ConditionBadge'
import { StatGem } from './StatGem'
import { StatusBanner } from './StatusBanner'
import { ThreatClock } from './ThreatClock'
import { Toast } from './Toast'

afterEach(() => vi.unstubAllGlobals())

describe('StatGem', () => {
  it('names its stat and value', () => {
    render(<StatGem stat="ac" value="16" />)
    expect(screen.getByRole('img', { name: 'Classe d’armure : 16' })).toBeInTheDocument()
  })
})

describe('ThreatClock', () => {
  it('fills its parts and makes the next one blink', () => {
    const { container } = render(<ThreatClock parts={6} filled={3} name="Le culte se réveille" />)
    expect(screen.getByRole('img', { name: 'Le culte se réveille : 3 sur 6' })).toBeInTheDocument()
    // 5 squares per part at 112 px.
    expect(container.querySelectorAll('[data-state="filled"]')).toHaveLength(15)
    expect(container.querySelectorAll('[data-state="next"]')).toHaveLength(5)
  })
})

describe('CardButton', () => {
  it('is a real button that acts on click and not when disabled', async () => {
    const onClick = vi.fn()
    const { rerender } = render(<CardButton title="Attaquer" gem={{ stat: 'atk', value: '+5' }} onClick={onClick} />)
    await userEvent.click(screen.getByRole('button', { name: /Attaquer/ }))
    expect(onClick).toHaveBeenCalledTimes(1)
    rerender(<CardButton title="Attaquer" onClick={onClick} disabled />)
    expect(screen.getByRole('button', { name: /Attaquer/ })).toBeDisabled()
  })
})

describe('ArcadeCluster', () => {
  it('plays the selected card, and waits while busy', async () => {
    const play = vi.fn()
    const end = vi.fn()
    const actions = {
      main: { label: 'Attaquer', icon: 'sword', onClick: play },
      left: { label: 'Sac', icon: 'flask' },
      right: { label: 'Fin du tour', icon: 'hourglass', onClick: end },
    } as const
    const { rerender } = render(<ArcadeCluster {...actions} />)
    await userEvent.click(screen.getByRole('button', { name: 'Attaquer' }))
    await userEvent.click(screen.getByRole('button', { name: 'Fin du tour' }))
    expect(play).toHaveBeenCalledTimes(1)
    expect(end).toHaveBeenCalledTimes(1)
    rerender(<ArcadeCluster {...actions} busy />)
    expect(screen.getByRole('button', { name: 'Attaquer' })).toBeDisabled()
  })
})

describe('ConditionBadge', () => {
  it('says the condition and its turns left', () => {
    render(
      <>
        <ConditionBadge icon="poison" name="Empoisonné" kind="bane" turns={3} />
        <ConditionBadge icon="blessed" name="Béni" kind="boon" turns={1} />
        <ConditionBadge icon="sleep" name="Endormi" kind="bane" />
      </>,
    )
    expect(screen.getByRole('img', { name: 'Empoisonné, 3 tours' })).toHaveAttribute('data-kind', 'bane')
    expect(screen.getByRole('img', { name: 'Béni, 1 tour' })).toHaveAttribute('data-kind', 'boon')
    expect(screen.getByRole('img', { name: 'Endormi' })).toBeInTheDocument()
  })
})

describe('StatusBanner', () => {
  it('announces what is happening', () => {
    render(<StatusBanner tone="you">À vous : que faites-vous ?</StatusBanner>)
    expect(screen.getByRole('status')).toHaveTextContent('À vous : que faites-vous ?')
  })
})

describe('Toast', () => {
  it('is put away by a swipe to the right, not by a short drag', () => {
    const onDismiss = vi.fn()
    render(
      <Toast tone="good" kicker="Butin · rare" onDismiss={onDismiss}>
        Écu de la garde rejoint ton sac.
      </Toast>,
    )
    const toast = screen.getByText('Butin · rare').closest('.gk-toast') as HTMLElement
    fireEvent.pointerDown(toast, { clientX: 10 })
    fireEvent.pointerMove(toast, { clientX: 40 })
    fireEvent.pointerUp(toast)
    expect(onDismiss).not.toHaveBeenCalled()
    fireEvent.pointerDown(toast, { clientX: 10 })
    fireEvent.pointerMove(toast, { clientX: 150 })
    fireEvent.pointerUp(toast)
    expect(onDismiss).toHaveBeenCalledTimes(1)
  })

  it('can be put away from the keyboard', async () => {
    const onDismiss = vi.fn()
    render(
      <Toast tone="info" kicker="Indice" onDismiss={onDismiss}>
        Le MJ vous montre le registre.
      </Toast>,
    )
    await userEvent.click(screen.getByRole('button', { name: 'Ranger' }))
    expect(onDismiss).toHaveBeenCalledTimes(1)
  })
})

describe('BottomPanel', () => {
  it('opens as a named dialog and closes on Escape', async () => {
    stubReducedMotion(true)
    const onOpenChange = vi.fn()
    render(
      <BottomPanel open onOpenChange={onOpenChange} title="Ton sac">
        <p>Une potion.</p>
      </BottomPanel>,
    )
    expect(await screen.findByRole('dialog', { name: 'Ton sac' })).toHaveTextContent('Une potion.')
    await userEvent.keyboard('{Escape}')
    expect(onOpenChange).toHaveBeenCalledWith(false)
  })

  it('renders nothing while closed', () => {
    render(
      <BottomPanel open={false} onOpenChange={() => {}} title="Ton sac">
        <p>Une potion.</p>
      </BottomPanel>,
    )
    expect(screen.queryByText('Une potion.')).toBeNull()
  })
})
