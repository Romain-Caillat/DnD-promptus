import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { bytes, mockApi, sentTo, stubPasskeys } from '@/test-utils'
import { SignInPage } from './SignInPage'

const CHALLENGE = {
  data: { ceremonyId: 'c1', options: { publicKey: { challenge: 'AAE', allowCredentials: [] } } },
}

const ASSERTION = {
  id: 'AQ',
  rawId: bytes(1),
  type: 'public-key',
  response: {
    authenticatorData: bytes(5),
    clientDataJSON: bytes(6),
    signature: bytes(7),
    userHandle: bytes(8),
  },
}

function renderAt() {
  render(
    <MemoryRouter initialEntries={['/connexion']}>
      <Routes>
        <Route path="/connexion" element={<SignInPage />} />
        <Route path="/" element={<p>accueil MJ</p>} />
        <Route path="/inscription" element={<p>création de compte</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('SignInPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('signs in with a passkey and lands on the GM home', async () => {
    const api = mockApi({
      'GET /api/auth/status': () => ({ status: 200, body: { data: { needsSetup: false } } }),
      'POST /api/auth/sign-in/options': () => ({ status: 200, body: CHALLENGE }),
      'POST /api/auth/sign-in': () => ({
        status: 200,
        body: { data: { id: 'g1', displayName: 'Romain' } },
      }),
    })
    stubPasskeys({ get: vi.fn().mockResolvedValue(ASSERTION) })
    renderAt()

    await userEvent.click(screen.getByRole('button', { name: 'Se connecter avec une passkey' }))

    expect(await screen.findByText('accueil MJ')).toBeInTheDocument()
    expect(sentTo(api, 'POST /api/auth/sign-in')).toEqual([
      {
        ceremonyId: 'c1',
        credential: expect.objectContaining({
          rawId: 'AQ',
          response: expect.objectContaining({ userHandle: 'CA' }),
        }),
      },
    ])
  })

  it('explains a closed prompt and sends nothing', async () => {
    const api = mockApi({
      'GET /api/auth/status': () => ({ status: 200, body: { data: { needsSetup: false } } }),
      'POST /api/auth/sign-in/options': () => ({ status: 200, body: CHALLENGE }),
    })
    stubPasskeys({
      get: vi.fn().mockRejectedValue(new DOMException('closed', 'NotAllowedError')),
    })
    renderAt()

    await userEvent.click(screen.getByRole('button', { name: 'Se connecter avec une passkey' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      "La passkey n'a pas été utilisée.",
    )
    expect(sentTo(api, 'POST /api/auth/sign-in')).toEqual([])
  })

  it('tells an unknown passkey apart from a closed prompt', async () => {
    mockApi({
      'GET /api/auth/status': () => ({ status: 200, body: { data: { needsSetup: false } } }),
      'POST /api/auth/sign-in/options': () => ({ status: 200, body: CHALLENGE }),
      'POST /api/auth/sign-in': () => ({
        status: 401,
        body: { error: { code: 'INVALID_PASSKEY', message: 'x' } },
      }),
    })
    stubPasskeys({ get: vi.fn().mockResolvedValue(ASSERTION) })
    renderAt()

    await userEvent.click(screen.getByRole('button', { name: 'Se connecter avec une passkey' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      "Cette passkey n'a pas été reconnue. Réessayez.",
    )
    expect(screen.queryByText('accueil MJ')).not.toBeInTheDocument()
  })

  it('sends a fresh server to the first account creation', async () => {
    mockApi({
      'GET /api/auth/status': () => ({ status: 200, body: { data: { needsSetup: true } } }),
    })
    stubPasskeys({})
    renderAt()

    await userEvent.click(await screen.findByRole('link', { name: 'Créer le premier compte' }))

    expect(await screen.findByText('création de compte')).toBeInTheDocument()
  })
})
