import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { ApiError } from '@/lib/api'
import { listCampaigns, type CampaignSummary } from '@/lib/campaigns'
import { pairScreen } from '@/lib/tv'

type State = { kind: 'loading' } | { kind: 'signed-out' } | { kind: 'error' } | { kind: 'ready'; campaigns: CampaignSummary[] }

const CODE_PATTERN = '[A-Za-z0-9]{4}'

/**
 * `/tv/jumeler` (session/pair-shared-screen): where the TV's QR leads the
 * GM's phone. Signed in, the GM picks the table the TV joins; the code
 * comes from the QR, or is typed.
 */
export function GmPairTvPage() {
  const { t } = useTranslation()
  const [params] = useSearchParams()
  const [code, setCode] = useState((params.get('code') ?? '').toUpperCase())
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [paired, setPaired] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const latest = useRef(0)

  useEffect(() => {
    const request = ++latest.current
    void (async () => {
      let next: State
      try {
        next = { kind: 'ready', campaigns: (await listCampaigns()).filter((c) => !c.archivedAt) }
      } catch (err) {
        next = err instanceof ApiError && err.status === 401 ? { kind: 'signed-out' } : { kind: 'error' }
      }
      if (request === latest.current) setState(next)
    })()
  }, [])

  async function pair(campaign: CampaignSummary) {
    setError(null)
    try {
      await pairScreen(campaign.id, code)
      setPaired(campaign.title)
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    }
  }

  return (
    <main className="surface-table mx-auto flex min-h-dvh max-w-md flex-col gap-4 p-4 text-chalk">
      <h1 className="type-title text-[24px]">{t('tv.gmPair.title')}</h1>
      {state.kind === 'loading' && <p role="status">{t('play.loading')}</p>}
      {state.kind === 'error' && <p role="alert">{t('play.error')}</p>}
      {state.kind === 'signed-out' && (
        <p>
          <Link to="/connexion" className="underline">
            {t('tv.gmPair.signIn')}
          </Link>
        </p>
      )}
      {paired && <p role="status">{t('tv.gmPair.done', { title: paired })}</p>}
      {state.kind === 'ready' && !paired && (
        <>
          <label className="flex flex-col gap-1.5">
            <span className="type-label">{t('tv.pair.codeLabel')}</span>
            <input
              value={code}
              onChange={(e) => setCode(e.target.value.toUpperCase())}
              maxLength={4}
              pattern={CODE_PATTERN}
              autoCapitalize="characters"
              className="rounded-button border border-line bg-transparent px-3 py-2 font-mono text-[28px] tracking-[.3em]"
            />
          </label>
          {error && (
            <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
              {t(`tv.errors.${error}`, { defaultValue: t('tv.errors.UNEXPECTED') })}
            </p>
          )}
          <span className="type-label">{t('tv.gmPair.which')}</span>
          <ul className="flex flex-col gap-2">
            {state.campaigns.map((c) => (
              <li key={c.id}>
                <button
                  type="button"
                  disabled={code.length !== 4}
                  onClick={() => void pair(c)}
                  className="w-full rounded-button border border-line px-3 py-3 text-left text-body disabled:opacity-40"
                >
                  {c.title}
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
    </main>
  )
}
