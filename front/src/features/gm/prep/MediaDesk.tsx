import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { StatusBanner } from '@/components/game/StatusBanner'
import { Btn, Panel } from '@/features/gm/live/ui'
import { ApiError } from '@/lib/api'
import { dollars } from '@/lib/money'
import { askImage, decideImage, drawMissing, fetchGmMedia, gmImageUrl, type GmMediaList, type MediaKind } from '@/lib/media'
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
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; story: Story; media: GmMediaList }

/**
 * The Atlas's « Médias » drawer — `media/generate-images-and-video`:
 * every scene, NPC, adversary and place of the campaign with its
 * pixel-art image, and every act with its introduction video. The GM
 * draws what is missing in one background batch (its cost said first),
 * then keeps or redraws each one; nothing reaches the table unkept.
 * `live` moves with the story and the background jobs (the Atlas hears
 * them). The images Claude brought from PixelLab and still waiting
 * (campaign/import-pixellab-assets) come first: whatever the map they
 * belong to, the GM keeps or discards them here.
 */
export function MediaDesk({ campaignId, live: heard }: { campaignId: string; live: number }) {
  const { t } = useTranslation()
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
        if (err instanceof ApiError && err.status === 404) setState({ kind: 'not-found' })
        else setState({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [campaignId, version, heard])

  if (state.kind !== 'ready') {
    return (
      <div className="flex flex-col gap-4">
        {state.kind === 'loading' && <p role="status">{t('prep.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('prep.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('prep.error')}</p>}
      </div>
    )
  }

  const { story, media } = state
  const { plan } = media
  const pixellab = media.assets.filter((a) => a.source === 'pixellab' && a.status === 'pending')
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
    <div className="flex flex-col gap-4">
      <h2 className="type-title text-[20px]">{t('prep.media.title')}</h2>
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

      {pixellab.length > 0 && (
        <section className="flex flex-col gap-2" aria-label={t('prep.media.pixellab')}>
          <h3 className="type-title text-[16px]">{t('prep.media.pixellab')}</h3>
          <p className="text-caption text-mute-soft">{t('prep.media.pixellabHint')}</p>
          <ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {pixellab.map((a) => (
              <li key={a.id} className="surface-slab flex items-center gap-3 p-3">
                <img
                  src={gmImageUrl(campaignId, a.id)}
                  alt={a.meta?.name ?? a.subject}
                  className="size-16 shrink-0 bg-black object-contain [image-rendering:pixelated]"
                />
                <div className="flex min-w-0 flex-col gap-1.5">
                  <span className="text-caption text-chalk">{a.meta?.name ?? a.subject}</span>
                  <span className="text-caption text-mute-soft">{t(`gmLive.media.kinds.${a.kind}`)}</span>
                  <div className="flex flex-wrap gap-1.5">
                    <Btn main disabled={busy} onClick={() => act(() => decideImage(campaignId, a.id, true))}>
                      {t('atlas.hd2d.keep')}
                    </Btn>
                    <Btn disabled={busy} onClick={() => act(() => decideImage(campaignId, a.id, false))}>
                      {t('atlas.hd2d.discard')}
                    </Btn>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        </section>
      )}

      {SECTIONS.map(({ kind, list }) => {
        const entities = story[list] ?? []
        if (entities.length === 0) return null
        return (
          <section key={kind} className="flex flex-col gap-2" aria-label={t(`gmLive.media.kinds.${kind}`)}>
            <h3 className="type-title text-[16px]">{t(`gmLive.media.kinds.${kind}`)}</h3>
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
    </div>
  )
}
