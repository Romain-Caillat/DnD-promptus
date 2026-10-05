import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, useNavigate } from 'react-router'
import { Button } from '@/components/ui/button'
import { fetchMe, signOut, type Gm } from '@/lib/auth'
import { CampaignsPanel } from './CampaignsPanel'
import { InvitesPanel } from './InvitesPanel'

type MeState = { kind: 'loading' } | { kind: 'signed-out' } | { kind: 'error' } | { kind: 'ready'; gm: Gm }

/**
 * `/` — the GM's home. Without a session it sends to `/connexion`;
 * it lists the campaigns, each opening its table.
 */
export function GmHomePage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [me, setMe] = useState<MeState>({ kind: 'loading' })

  useEffect(() => {
    let live = true
    fetchMe().then(
      (gm) => {
        if (live) setMe(gm ? { kind: 'ready', gm } : { kind: 'signed-out' })
      },
      () => {
        if (live) setMe({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [])

  if (me.kind === 'signed-out') return <Navigate to="/connexion" replace />

  async function leave() {
    try {
      await signOut()
      navigate('/connexion', { replace: true })
    } catch {
      setMe({ kind: 'error' })
    }
  }

  return (
    <main className="mx-auto flex min-h-dvh max-w-2xl flex-col gap-8 p-6">
      {me.kind === 'loading' && <p role="status">{t('gm.home.loading')}</p>}
      {me.kind === 'error' && <p role="alert">{t('auth.errors.generic')}</p>}
      {me.kind === 'ready' && (
        <>
          <header className="flex flex-wrap items-center justify-between gap-3">
            <h1 className="text-2xl font-semibold tracking-tight">
              {t('gm.home.greeting', { name: me.gm.displayName })}
            </h1>
            <Button variant="outline" onClick={() => void leave()}>
              {t('gm.home.signOut')}
            </Button>
          </header>
          <CampaignsPanel />
          <InvitesPanel />
        </>
      )}
    </main>
  )
}
