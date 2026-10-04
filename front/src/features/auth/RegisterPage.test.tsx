import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { bytes, mockApi, sentTo, stubPasskeys } from '@/test-utils'
import { RegisterPage } from './RegisterPage'

const CHALLENGE = {
  data: {
    ceremonyId: 'r1',
    options: {
      publicKey: {
        challenge: 'AAE',
        user: { id: 'Ag', name: 'Marc', displayName: 'Marc' },
        rp: { id: 'localhost', name: 'Promptus' },
      },
    },
  },
}

const ATTESTATION = {
  id: 'AQI',
  rawId: bytes(1, 2),
  type: 'public-key',
  response: { attestationObject: bytes(3), clientDataJSON: bytes(4) },
}

function renderAt(url: string) {
  render(
    <MemoryRouter initialEntries={[url]}>
      <Routes>
        <Route path="/inscription" element={<RegisterPage />} />
        <Route path="/" element={<p>accueil MJ</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('RegisterPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('creates the account from an invitation link and lands on the GM home', async () => {
    const api = mockApi({
      'GET /api/auth/status': () => ({ status: 200, body: { data: { needsSetup: false } } }),
      'POST /api/auth/register/options': () => ({ status: 200, body: CHALLENGE }),
      'POST /api/auth/register': () => ({
        status: 201,
        body: { data: { id: 'g2', displayName: 'Marc' } },
      }),
    })
    const create = vi.fn().mockResolvedValue(ATTESTATION)
    stubPasskeys({ create })
    renderAt('/inscription?code=invite-123')

    expect(screen.getByLabelText("Code d'invitation ou de démarrage")).toHaveValue('invite-123')
    await userEvent.type(screen.getByLabelText('Nom affiché'), '  Marc ')
    await userEvent.click(screen.getByRole('button', { name: 'Créer mon compte avec une passkey' }))

    expect(await screen.findByText('accueil MJ')).toBeInTheDocument()
    expect(sentTo(api, 'POST /api/auth/register/options')).toEqual([
      { code: 'invite-123', displayName: 'Marc' },
    ])
    expect(sentTo(api, 'POST /api/auth/register')).toEqual([
      {
        ceremonyId: 'r1',
        credential: expect.objectContaining({ rawId: 'AQI' }),
      },
    ])
  })

  it('refuses a wrong code before any passkey is created', async () => {
    mockApi({
      'GET /api/auth/status': () => ({ status: 200, body: { data: { needsSetup: true } } }),
      'POST /api/auth/register/options': () => ({
        status: 403,
        body: { error: { code: 'INVALID_REGISTRATION_CODE', message: 'x' } },
      }),
    })
    const create = vi.fn()
    stubPasskeys({ create })
    renderAt('/inscription')

    expect(
      await screen.findByText(
        'Premier compte du serveur : le code de démarrage est affiché dans le journal du serveur.',
      ),
    ).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText("Code d'invitation ou de démarrage"), 'guess')
    await userEvent.type(screen.getByLabelText('Nom affiché'), 'Mallory')
    await userEvent.click(screen.getByRole('button', { name: 'Créer mon compte avec une passkey' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      "Ce code n'est pas valable : il est faux, déjà utilisé, révoqué ou expiré.",
    )
    expect(create).not.toHaveBeenCalled()
  })
})
