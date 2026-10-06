import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useParams } from 'react-router'
import { CardButton } from '@/components/game/CardButton'
import { creatorPath } from '@/lib/creator'
import { cn } from '@/lib/utils'
import { fetchPlayerHome, type CharacterView, type PlayerHome } from '@/lib/play'
import { CharacterSummary } from './creator/ReviewStep'
import { RulesEntry } from './rules/RulesEntry'

type HomeState =
  | { kind: 'loading' }
  | { kind: 'not-joined' }
  | { kind: 'error' }
  | { kind: 'ready'; home: PlayerHome }

/**
 * `/partie/:campaignId` — a player's home in one campaign, phone first:
 * the campaign, their seat and their character — drawn, with its
 * numbers, once made — and the way into the creator while the sheet is
 * theirs to edit. Everything comes from the server's player projection
 * (`GET /api/play/…/me`).
 */
export function PlayerHomePage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
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
            {character.sheet.look ? (
              <CharacterSummary
                sheet={character.sheet}
                look={character.sheet.look}
                subtitle={[character.peopleName, character.className, t('creator.review.level')]
                  .filter(Boolean)
                  .join(' · ')}
                stats={character.stats}
                over={0}
                gmName={campaign.gmName}
              />
            ) : (
              <span className="type-title text-[22px]">{character.sheet.name || t('play.unnamed')}</span>
            )}
            <p className="text-body text-chalk-soft">{t(`play.status.${character.status}`)}</p>
            {character.gmNote && (
              <div className="rounded-button bg-ivory px-3.5 py-3 text-body text-ink shadow-ivory-flat">
                <span className="type-label block text-ink-soft">{t('play.gmNote')}</span>
                {character.gmNote}
              </div>
            )}
            <CreatorButton character={character} onOpen={() => navigate(creatorPath(campaignId))} />
          </div>
        ) : (
          <p className="text-body text-chalk-soft">{t('play.spectator')}</p>
        )}
        <RulesEntry campaignId={campaignId} />
      </section>
      <p className="px-4 pb-8 text-caption text-mute">{t('play.keep')}</p>
    </main>
  )
}

/** The way into the creator, while the sheet is the player's to edit. */
function CreatorButton({ character, onOpen }: { character: CharacterView; onOpen: () => void }) {
  const { t } = useTranslation()
  if (character.status === 'returned') {
    return <CardButton title={t('play.fix')} subtitle={t('play.fixHint')} onClick={onOpen} />
  }
  if (character.status !== 'draft') return null
  const started = Boolean(character.sheet.look || character.sheet.name)
  return (
    <CardButton
      title={t(started ? 'play.resume' : 'play.create')}
      subtitle={t('play.createHint')}
      onClick={onOpen}
    />
  )
}
