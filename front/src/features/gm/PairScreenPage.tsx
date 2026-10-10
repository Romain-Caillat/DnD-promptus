import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, useNavigate, useSearchParams } from 'react-router'
import { ApiError } from '@/lib/api'
import { listCampaigns, type CampaignSummary } from '@/lib/campaigns'
import { pairScreen } from '@/lib/screens'

type State = { kind: 'loading' } | { kind: 'signed-out' } | { kind: 'error' } | { kind: 'ready'; campaigns: CampaignSummary[] }

/**
 * `/tv/jumeler?code=K7QF` — where the QR shown by a TV leads
 * (session/pair-shared-screen): the GM, signed in on their phone or
 * laptop, picks the table that TV is for, and lands on that evening's
 * screen. The same pairing as typing the code there.
 */
export function PairScreenPage() {
  const { t } = useTranslation()
  const [params] = useSearchParams()
  const navigate = useNavigate()
  const code = (params.get('code') ?? '').toUpperCase()
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    listCampaigns()
      .then((campaigns) => setState({ kind: 'ready', campaigns: campaigns.filter((c) => !c.archivedAt) }))
      .catch((err: unknown) =>
        setState(err instanceof ApiError && err.status === 401 ? { kind: 'signed-out' } : { kind: 'error' }),
      )
  }, [])

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />
  return (
    <main className="surface-table flex min-h-dvh flex-col gap-4 p-6 text-chalk">
      <h1 className="type-title text-heading">{t('pairScreen.title', { code })}</h1>
      <p className="text-body text-mute-soft">{t('pairScreen.which')}</p>
      {state.kind === 'loading' && <p role="status">{t('pairScreen.loading')}</p>}
      {state.kind === 'error' && <p role="alert">{t('pairScreen.error')}</p>}
      {error && (
        <p role="alert" className="text-body text-stat-atk">
          {t(`gmLive.screens.errors.${error}`, { defaultValue: t('gmLive.screens.errors.UNEXPECTED') })}
        </p>
      )}
      {state.kind === 'ready' && (
        <ul className="flex flex-col gap-2">
          {state.campaigns.map((c) => (
            <li key={c.id}>
              <button
                type="button"
                className="pixel-choice w-full py-3 pr-4 pb-3.5 text-left text-body font-bold"
                onClick={async () => {
                  setError(null)
                  try {
                    await pairScreen(c.id, code)
                    navigate(`/campagnes/${encodeURIComponent(c.id)}/soiree`)
                  } catch (err) {
                    setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
                  }
                }}
              >
                {c.title}
              </button>
            </li>
          ))}
        </ul>
      )}
    </main>
  )
}
