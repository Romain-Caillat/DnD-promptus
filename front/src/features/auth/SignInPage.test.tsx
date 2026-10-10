import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi, sentTo } from '@/test-utils'
import { SignInPage } from './SignInPage'

function renderAt() {
  render(
    <MemoryRouter initialEntries={['/connexion']}>
      <Routes>
        <Route path="/connexion" element={<SignInPage />} />
        <Route path="/" element={<p>accueil MJ</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

const GM = { status: 200, body: { data: { id: 'g1', displayName: 'Romain' } } }

async function askCode(email = 'romain@example.org') {
  await userEvent.type(screen.getByLabelText('Adresse email'), email)
  await userEvent.click(screen.getByRole('button', { name: 'Recevoir un code' }))
}

describe('SignInPage', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    localStorage.clear()
  })

  it('signs in with the code received and lands on the GM home', async () => {
    const api = mockApi({
      'POST /api/auth/code': () => ({ status: 204 }),
      'POST /api/auth/verify': () => GM,
    })
    renderAt()

    await askCode()
    expect(await screen.findByText(/vient d’être envoyé à romain@example.org/)).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText('Code reçu par email'), '12 34-56')
    await userEvent.click(screen.getByRole('button', { name: 'Se connecter' }))

    expect(await screen.findByText('accueil MJ')).toBeInTheDocument()
    expect(sentTo(api, 'POST /api/auth/code')).toEqual([{ email: 'romain@example.org' }])
    expect(sentTo(api, 'POST /api/auth/verify')).toEqual([
      { email: 'romain@example.org', code: '123456' },
    ])
  })

  it('asks a new address for its display name, then creates the account', async () => {
    const api = mockApi({
      'POST /api/auth/code': () => ({ status: 204 }),
      'POST /api/auth/verify': (sent) =>
        (sent as { displayName?: string }).displayName
          ? { status: 201, body: GM.body }
          : { status: 409, body: { error: { code: 'DISPLAY_NAME_REQUIRED', message: 'x' } } },
    })
    renderAt()

    await askCode()
    await userEvent.type(await screen.findByLabelText('Code reçu par email'), '123456')
    await userEvent.click(screen.getByRole('button', { name: 'Se connecter' }))
    await userEvent.type(await screen.findByLabelText('Nom affiché'), 'Romain')
    await userEvent.click(screen.getByRole('button', { name: 'Créer mon compte MJ' }))

    expect(await screen.findByText('accueil MJ')).toBeInTheDocument()
    expect(sentTo(api, 'POST /api/auth/verify')).toEqual([
      { email: 'romain@example.org', code: '123456' },
      { email: 'romain@example.org', code: '123456', displayName: 'Romain' },
    ])
  })

  it('explains a wrong code and stays on the page', async () => {
    mockApi({
      'POST /api/auth/code': () => ({ status: 204 }),
      'POST /api/auth/verify': () => ({
        status: 401,
        body: { error: { code: 'INVALID_CODE', message: 'x' } },
      }),
    })
    renderAt()

    await askCode()
    await userEvent.type(await screen.findByLabelText('Code reçu par email'), '000000')
    await userEvent.click(screen.getByRole('button', { name: 'Se connecter' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Ce code n’est pas bon')
    expect(screen.queryByText('accueil MJ')).not.toBeInTheDocument()
  })

  it('says when a code was asked for too soon', async () => {
    mockApi({
      'POST /api/auth/code': () => ({
        status: 429,
        body: { error: { code: 'CODE_TOO_SOON', message: 'x' } },
      }),
    })
    renderAt()

    await askCode()

    expect(await screen.findByRole('alert')).toHaveTextContent('Attends une minute')
    expect(screen.getByLabelText('Adresse email')).toBeInTheDocument()
  })

  it('remembers the last address on this device', async () => {
    mockApi({ 'POST /api/auth/code': () => ({ status: 204 }) })
    renderAt()
    await askCode('marc@example.org')
    await screen.findByLabelText('Code reçu par email')
    await userEvent.click(screen.getByRole('button', { name: 'Changer d’adresse' }))

    expect(screen.getByLabelText('Adresse email')).toHaveValue('marc@example.org')
  })
})
