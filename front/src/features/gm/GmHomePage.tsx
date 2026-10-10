import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, useNavigate } from 'react-router'
import { Button } from '@/components/ui/button'
import { fetchMe, signOut, type Gm } from '@/lib/auth'
import { ThemeSetting } from '@/features/theme/ThemeSetting'
import { CampaignsPanel } from './CampaignsPanel'
import { ClaudeAccessPanel } from './ClaudeAccessPanel'

type MeState = { kind: 'loading' } | { kind: 'signed-out' } | { kind: 'error' } | { kind: 'ready'; gm: Gm }

/**
 * `/` — the GM's home. Without a session it sends to `/connexion`;
 * it lists the campaigns, each reopening its campaign page.
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
    <main className="surface-table flex min-h-dvh flex-col text-chalk">
      {me.kind === 'loading' && (
        <p role="status" className="p-6">
          {t('gm.home.loading')}
        </p>
      )}
      {me.kind === 'error' && (
        <p role="alert" className="p-6">
          {t('auth.errors.generic')}
        </p>
      )}
      {me.kind === 'ready' && (
        <>
          <header className="flex flex-wrap items-center gap-4 border-b border-line px-5 py-3 text-caption text-mute-soft">
            <span className="type-title text-[12px] tracking-[0.3em] text-chalk uppercase">{t('app.name')}</span>
            <p>{t('gm.home.greeting', { name: me.gm.displayName })}</p>
            <ThemeSetting className="ml-auto" />
            <Button variant="outline" size="sm" onClick={() => void leave()}>
              {t('gm.home.signOut')}
            </Button>
          </header>
          <div className="mx-auto flex w-full max-w-6xl flex-col gap-10 p-5">
            <CampaignsPanel gmName={me.gm.displayName} />
            <div className="surface-slab max-w-2xl p-4">
              <ClaudeAccessPanel />
            </div>
          </div>
        </>
      )}
    </main>
  )
}
