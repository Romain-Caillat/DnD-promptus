import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useParams } from 'react-router'
import { StatusBanner } from '@/components/game/StatusBanner'
import { Btn, Panel } from '@/features/gm/live/ui'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { ApiError } from '@/lib/api'
import { dollars } from '@/lib/generation'
import { askImage, decideImage, drawMissing, fetchGmMedia, type GmMediaList, type MediaKind } from '@/lib/media'
import { fetchReview, nameOf, type Story } from '@/lib/prep'
import { SubjectCard } from './SubjectCard'

/** The kinds this page draws, in the order the GM reads them, with the story list each comes from. */
const SECTIONS = [
  { kind: 'intro', list: 'acts' },
  { kind: 'scene', list: 'nodes' },
  { kind: 'npc', list: 'npcs' },
  { kind: 'adversary', list: 'adversaries' },
  { kind: 'location', list: 'locations' },
] as const satisfies readonly { kind: MediaKind; list: keyof Story }[]

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; story: Story; media: GmMediaList }

/**
 * `/campagnes/:campaignId/medias` — `media/generate-images-and-video`:
 * every scene, NPC, adversary and place of the campaign with its
 * pixel-art image, and every act with its introduction video. The GM
 * draws what is missing in one background batch (its cost said first),
 * then keeps or redraws each one; nothing reaches the table unkept.
 */
export function MediaPage() {
  const { t } = useTranslation()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [version, setVersion] = useState(0)
  const [videos, setVideos] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    Promise.all([fetchReview(campaignId), fetchGmMedia(campaignId)]).then(
      ([review, media]) => {
        if (live) setState({ kind: 'ready', story: review.story, media })
      },
      (err: unknown) => {
        if (!live) return
        if (err instanceof ApiError && err.status === 401) setState({ kind: 'signed-out' })
        else if (err instanceof ApiError && err.status === 404) setState({ kind: 'not-found' })
        else setState({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [campaignId, version])

  useLiveChanges(campaignId, (topics) => {
    if (topics.includes('desk') || topics.includes('story')) setVersion((v) => v + 1)
  })

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />
  const back = (
    <Link className="text-caption text-mute-soft underline underline-offset-4" to={`/campagnes/${campaignId}`}>
      {t('prep.back')}
    </Link>
  )
  if (state.kind !== 'ready') {
    return (
      <main className="surface-table flex min-h-dvh flex-col gap-4 p-6 text-chalk">
        {back}
        {state.kind === 'loading' && <p role="status">{t('prep.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('prep.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('prep.error')}</p>}
      </main>
    )
  }

  const { story, media } = state
  const { plan } = media
  const left = Math.max(0, plan.spending.budgetMicros - plan.spending.spentMicros)
  const count = plan.images.length + (videos ? plan.videos.length : 0)
  const estimate = plan.images.length * plan.imageMicros + (videos ? plan.videos.length * plan.videoMicros : 0)

  async function act(run: () => Promise<void>) {
    setBusy(true)
    setError(null)
    try {
      await run()
      setVersion((v) => v + 1)
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'action')
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className="surface-table flex min-h-dvh flex-col gap-4 p-5 text-chalk">
      {back}
      <h1 className="type-title text-[22px]">{t('prep.media.title')}</h1>
      {!plan.configured && <StatusBanner tone="warn">{t('prep.generate.errors.AI_NOT_CONFIGURED')}</StatusBanner>}
      {error && (
        <p role="alert" className="text-body text-stat-atk">
          {t(`prep.media.errors.${error}`, { defaultValue: t('prep.generate.errors.action') })}
        </p>
      )}

      <Panel title={t('prep.media.batchTitle')}>
        <p className="text-body">
          {t('prep.media.missing', { images: plan.images.length, videos: plan.videos.length })}
        </p>
        <label className="flex items-center gap-2 text-body">
          <input
            type="checkbox"
            checked={videos}
            disabled={plan.videos.length === 0}
            onChange={(e) => setVideos(e.target.checked)}
          />
          {t('prep.media.withVideos', { count: plan.videos.length })}
        </label>
        <p className="text-caption text-mute-soft">
          {t('prep.generate.cost', { estimate: dollars(estimate), left: dollars(left) })}
        </p>
        <Btn
          main
          disabled={busy || plan.running || count === 0 || !plan.configured}
          onClick={() => void act(async () => void (await drawMissing(campaignId, videos)))}
        >
          {plan.running ? t('prep.media.running') : t('prep.media.drawAll', { count })}
        </Btn>
      </Panel>

      {SECTIONS.map(({ kind, list }) => {
        const entities = story[list] ?? []
        if (entities.length === 0) return null
        return (
          <section key={kind} className="flex flex-col gap-2" aria-label={t(`gmLive.media.kinds.${kind}`)}>
            <h2 className="type-title text-[17px]">{t(`gmLive.media.kinds.${kind}`)}</h2>
            <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
              {entities.map((e) => (
                <SubjectCard
                  key={e.id}
                  campaignId={campaignId}
                  kind={kind}
                  name={nameOf(e)}
                  assets={media.assets.filter((a) => a.kind === kind && a.subject === e.id)}
                  disabled={busy || !plan.configured}
                  onAsk={(direction) => act(async () => void (await askImage(campaignId, kind, e.id, direction)))}
                  onDecide={(asset, approve) => act(() => decideImage(campaignId, asset, approve))}
                />
              ))}
            </div>
          </section>
        )
      })}
    </main>
  )
}
