import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { LiveScreen } from '@/lib/evening'
import type { Heard, Recorder } from '@/lib/voice'
import { CopilotPanel } from './CopilotPanel'

const SCREEN = {
  session: { id: 's1', number: 1, status: 'live' },
  scene: {
    npcs: [{ id: 'pnj_goulven', name: 'Marguerite Goulven' }],
  },
  drafts: [],
  companions: [],
  ai: { configured: true, spending: { budgetMicros: 1_000_000, spentMicros: 0 } },
} as unknown as LiveScreen

const HEARD: Heard = {
  transcript: 'Que répond Marguerite Goulven si on lui parle du trésor ?',
  seconds: 3,
  draft: {
    id: 'd1',
    kind: 'npc',
    prompt: 'Que répond Marguerite Goulven si on lui parle du trésor ?',
    answer: { narration: '', npcLines: [], suggestions: [], gmNote: null },
    status: 'draft',
    createdAt: '',
  },
}

function panel(recorder: Recorder, onDictate = vi.fn(async () => HEARD)) {
  render(
    <CopilotPanel
      screen={SCREEN}
      recorder={recorder}
      onAsk={vi.fn(async () => {})}
      onDictate={onDictate}
      onShow={vi.fn()}
      onDismiss={vi.fn()}
      onReveal={vi.fn()}
    />,
  )
  return onDictate
}

describe('the co-GM by voice', () => {
  it('records on one touch, sends on the next with the chosen kind, and shows what it heard', async () => {
    const user = userEvent.setup()
    const stop = vi.fn(async () => ({ audio: 'UklGRg==', seconds: 3 }))
    const recorder: Recorder = { supported: true, start: vi.fn(async () => ({ stop, cancel: vi.fn() })) }
    const onDictate = panel(recorder)

    await user.click(screen.getByRole('button', { name: 'Faire parler' }))
    await user.selectOptions(screen.getByLabelText('PNJ'), 'pnj_goulven')
    await user.click(screen.getByRole('button', { name: /Parler au co-MJ/ }))
    const mic = screen.getByRole('button', { name: /J'écoute/ })
    expect(mic).toHaveAttribute('aria-pressed', 'true')
    expect(onDictate).not.toHaveBeenCalled()

    await user.click(mic)
    expect(stop).toHaveBeenCalledOnce()
    expect(onDictate).toHaveBeenCalledWith('UklGRg==', 'npc', 'pnj_goulven')
    expect(await screen.findByText(/« Que répond Marguerite Goulven/)).toBeInTheDocument()

    // « Changer »: the words go to the text field, to correct and ask again.
    await user.click(screen.getByRole('button', { name: 'Changer' }))
    expect(screen.getByLabelText('Ce que disent les joueurs, votre question…')).toHaveValue(HEARD.transcript)
  })

  it('says why when the microphone is refused, and offers no button without one', async () => {
    const user = userEvent.setup()
    panel({ supported: true, start: vi.fn(async () => Promise.reject(new Error('NotAllowedError'))) })
    await user.click(screen.getByRole('button', { name: /Parler au co-MJ/ }))
    expect(screen.getByRole('alert')).toHaveTextContent('Le micro est refusé')
  })

  it('offers no microphone where the page cannot have one', () => {
    panel({ supported: false, start: vi.fn() })
    expect(screen.queryByRole('button', { name: /Parler au co-MJ/ })).not.toBeInTheDocument()
    expect(screen.getByText(/la dictée demande une page en HTTPS/)).toBeInTheDocument()
  })
})
