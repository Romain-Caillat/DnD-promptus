import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ApiToken } from '@/lib/claudeAccess'
import { mockApi, sentTo } from '@/test-utils'
import { ClaudeAccessPanel } from './ClaudeAccessPanel'

const USED = {
  id: 't1',
  name: 'Claude sur mon Mac',
  createdAt: '2026-10-08T10:00:00Z',
  lastUsedAt: '2026-10-08T12:30:00Z',
}
const FRESH = { id: 't2', name: 'Claude Code', createdAt: '2026-10-08T13:00:00Z', lastUsedAt: null }

describe('ClaudeAccessPanel', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('mints a named token shown once, inside the Claude snippets, then revokes one', async () => {
    let tokens: ApiToken[] = [USED]
    const api = mockApi({
      'GET /api/gm-tokens': () => ({ status: 200, body: { data: tokens } }),
      'POST /api/gm-tokens': () => {
        tokens = [FRESH, USED]
        return { status: 201, body: { data: { ...FRESH, secret: 'promptus_bbbbbbbb' } } }
      },
      'DELETE /api/gm-tokens/t1': () => {
        tokens = [FRESH]
        return { status: 204 }
      },
    })
    render(<ClaudeAccessPanel />)

    expect(await screen.findByText('Claude sur mon Mac')).toBeInTheDocument()
    expect(screen.getByText(/dernière utilisation le/)).toBeInTheDocument()
    // Before a token is minted, the snippets carry a placeholder.
    expect(screen.getByLabelText('Configuration de Claude Desktop')).toHaveTextContent('COLLE_TON_JETON_ICI')

    await userEvent.type(screen.getByLabelText('Nom du jeton'), '  Claude Code ')
    await userEvent.click(screen.getByRole('button', { name: 'Créer un jeton' }))

    expect(await screen.findByLabelText('Jeton pour Claude')).toHaveValue('promptus_bbbbbbbb')
    expect(sentTo(api, 'POST /api/gm-tokens')).toEqual([{ name: 'Claude Code' }])
    expect(await screen.findByText(/jamais utilisé/)).toBeInTheDocument()
    const desktop = JSON.parse(screen.getByLabelText('Configuration de Claude Desktop').textContent!)
    expect(desktop.mcpServers.promptus.env).toEqual({
      PROMPTUS_URL: window.location.origin,
      PROMPTUS_TOKEN: 'promptus_bbbbbbbb',
    })
    expect(screen.getByLabelText('Commande pour Claude Code')).toHaveTextContent(
      `claude mcp add promptus -s user -e PROMPTUS_URL=${window.location.origin} -e PROMPTUS_TOKEN=promptus_bbbbbbbb -- bun run`,
    )

    const item = screen.getByText('Claude sur mon Mac').closest('li')!
    await userEvent.click(within(item).getByRole('button', { name: 'Révoquer le jeton Claude sur mon Mac' }))
    await vi.waitFor(() => expect(screen.queryByText('Claude sur mon Mac')).not.toBeInTheDocument())
    expect(sentTo(api, 'DELETE /api/gm-tokens/t1')).toHaveLength(1)
  })

  it('asks for a name before minting', async () => {
    const api = mockApi({ 'GET /api/gm-tokens': () => ({ status: 200, body: { data: [] } }) })
    render(<ClaudeAccessPanel />)

    expect(await screen.findByText('Aucun jeton pour l’instant.')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Créer un jeton' }))

    expect(screen.getByRole('alert')).toHaveTextContent('Donne un nom au jeton')
    expect(sentTo(api, 'POST /api/gm-tokens')).toHaveLength(0)
  })
})
