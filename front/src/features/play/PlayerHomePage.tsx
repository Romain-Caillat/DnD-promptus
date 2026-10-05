import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'
import { cn } from '@/lib/utils'
import { fetchPlayerHome, type PlayerHome } from '@/lib/play'

type HomeState =
  | { kind: 'loading' }
  | { kind: 'not-joined' }
  | { kind: 'error' }
  | { kind: 'ready'; home: PlayerHome }

/**
 * `/partie/:campaignId` — a player's home in one campaign, phone first:
 * the campaign, their seat and where their character stands. Everything
 * comes from the server's player projection (`GET /api/play/…/me`).
 */
export function PlayerHomePage() {
  const { t } = useTranslation()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<HomeState>({ kind: 'loading' })

  useEffect(() => {
    let live = true
    fetchPlayerHome(campaignId).then(
      (home) => {
        if (live) setState(home ? { kind: 'ready', home } : { kind: 'not-joined' })
      },
      () => {
        if (live) setState({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [campaignId])

  if (state.kind !== 'ready') {
    return (
      <main className="surface-table mx-auto flex min-h-dvh max-w-md flex-col p-4 text-chalk">
        {state.kind === 'loading' && <p role="status">{t('play.loading')}</p>}
        {state.kind === 'not-joined' && <p role="alert">{t('play.notJoined')}</p>}
        {state.kind === 'error' && <p role="alert">{t('play.error')}</p>}
      </main>
    )
  }

  const { me, campaign, character } = state.home
  const bannerKey = me.role === 'spectator' || !character ? 'spectator' : character.status
  const good = bannerKey === 'validated' || bannerKey === 'draft'

  return (
    <main className="surface-table mx-auto flex min-h-dvh max-w-md flex-col text-chalk">
      <p
        role="status"
        className={cn(
          'mx-3 mt-4 rounded-button px-3.5 py-2.5 text-body font-bold',
          good && 'bg-ivory text-ink shadow-ivory-flat',
          bannerKey === 'submitted' && 'border-[1.5px] border-dashed border-line-dashed bg-table text-chalk-soft',
          bannerKey === 'returned' && 'border-[1.5px] border-stat-atk bg-table text-chalk',
          bannerKey === 'spectator' && 'border border-line bg-surface text-chalk-soft',
        )}
      >
        {t(`play.banner.${bannerKey}`)}
      </p>
      <section className="flex flex-1 flex-col gap-3 p-4">
        <span className="type-label">{t('play.hello', { nickname: me.nickname, gm: campaign.gmName })}</span>
        <h1 className="type-title text-heading">{campaign.title}</h1>
        {campaign.playerHook && (
          <p className="type-narration text-[18px] leading-snug text-chalk-soft">{campaign.playerHook}</p>
        )}
        {character ? (
          <div className="surface-slab flex flex-col gap-2 p-3.5">
            <span className="type-label">{t('play.character')}</span>
            <span className="type-title text-[22px]">{character.sheet.name || t('play.unnamed')}</span>
            <p className="text-body text-chalk-soft">{t(`play.status.${character.status}`)}</p>
            {character.gmNote && (
              <div className="rounded-button bg-ivory px-3.5 py-3 text-body text-ink shadow-ivory-flat">
                <span className="type-label block text-ink-soft">{t('play.gmNote')}</span>
                {character.gmNote}
              </div>
            )}
          </div>
        ) : (
          <p className="text-body text-chalk-soft">{t('play.spectator')}</p>
        )}
      </section>
      <p className="px-4 pb-8 text-caption text-mute">{t('play.keep')}</p>
    </main>
  )
}
