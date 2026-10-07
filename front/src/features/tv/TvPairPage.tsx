import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useSearchParams } from 'react-router'
import { ApiError } from '@/lib/api'
import { pollPairing, qrUrl, rememberTable, rememberedTable, startPairing, type Pairing } from '@/lib/tv'

/** How often a waiting TV asks whether the GM typed its code. */
const POLL_MS = 2_000

type State = { kind: 'starting' } | { kind: 'waiting'; pairing: Pairing } | { kind: 'error' }

/**
 * `/tv` (session/pair-shared-screen, planche « Lancer », moment 1): the
 * TV shows a code and a QR, then joins the table once the GM typed it.
 * A TV that already joined a table goes back to it. A window the GM
 * opens to share comes with its secret (`?cle=`), already paired.
 */
export function TvPairPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const windowSecret = params.get('cle')
  const [state, setState] = useState<State>({ kind: 'starting' })
  const attempt = useRef(0)

  useEffect(() => {
    const known = rememberedTable()
    if (known && !windowSecret) {
      void navigate(`/tv/${known}`, { replace: true })
      return
    }
    const mine = ++attempt.current
    let timer: ReturnType<typeof setTimeout> | undefined
    const begin = async () => {
      let pairing: Pairing | null
      try {
        pairing = windowSecret ? { code: '', secret: windowSecret } : await startPairing()
      } catch {
        pairing = null
      }
      if (mine !== attempt.current) return
      if (!pairing) {
        setState({ kind: 'error' })
        return
      }
      if (!windowSecret) setState({ kind: 'waiting', pairing })
      const ask = async () => {
        try {
          const answer = await pollPairing(pairing.secret)
          if (mine !== attempt.current) return
          if (answer.state === 'paired') {
            rememberTable(answer.campaign)
            void navigate(`/tv/${answer.campaign}`, { replace: true })
            return
          }
        } catch (err) {
          if (mine !== attempt.current) return
          // Expired: a fresh code.
          if (err instanceof ApiError && err.status === 404) {
            if (windowSecret) setState({ kind: 'error' })
            else void begin()
            return
          }
        }
        timer = setTimeout(() => void ask(), POLL_MS)
      }
      void ask()
    }
    void begin()
    return () => {
      attempt.current += 1
      if (timer) clearTimeout(timer)
    }
  }, [navigate, windowSecret])

  return (
    <main className="surface-table flex min-h-dvh flex-col items-center justify-center gap-8 p-8 text-center text-chalk">
      <span className="type-label tracking-[.3em]">{t('tv.brand')}</span>
      {state.kind === 'starting' && <p role="status">{t('tv.pair.starting')}</p>}
      {state.kind === 'error' && <p role="alert">{t('tv.pair.error')}</p>}
      {state.kind === 'waiting' && (
        <div className="flex flex-wrap items-center justify-center gap-10">
          <img
            src={qrUrl(state.pairing.secret)}
            alt={t('tv.pair.qr')}
            className="size-60 rounded-lg bg-chalk p-2"
          />
          <div className="flex flex-col items-start gap-3">
            <span className="type-label">{t('tv.pair.codeLabel')}</span>
            <div className="flex gap-2.5" aria-label={t('tv.pair.code', { code: state.pairing.code })}>
              {[...state.pairing.code].map((c, i) => (
                <b
                  key={i}
                  className="grid h-20 w-16 place-items-center rounded-xl bg-ivory font-mono text-[44px] text-ink shadow-ivory-flat"
                >
                  {c}
                </b>
              ))}
            </div>
            <span className="text-body text-chalk-soft">{t('tv.pair.hint')}</span>
          </div>
        </div>
      )}
    </main>
  )
}
