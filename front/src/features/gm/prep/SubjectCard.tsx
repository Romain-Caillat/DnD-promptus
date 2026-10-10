import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Btn, field } from '@/features/gm/live/ui'
import { gmImageUrl, type Asset, type MediaKind } from '@/lib/media'

/** The image or video itself, as the GM sees it. */
function Shown({ campaignId, asset, name }: { campaignId: string; asset: Asset; name: string }) {
  const src = gmImageUrl(campaignId, asset.id)
  if (asset.kind === 'intro') {
    return <video src={src} controls playsInline preload="metadata" className="w-full rounded-md" aria-label={name} />
  }
  return <img src={src} alt={name} className="w-full rounded-md [image-rendering:pixelated]" />
}

/** One subject: what the table may see of it, what waits for the GM, and a way to draw it again. */
export function SubjectCard({
  campaignId,
  kind,
  name,
  assets,
  disabled,
  onAsk,
  onDecide,
}: {
  campaignId: string
  kind: MediaKind
  name: string
  assets: Asset[]
  disabled: boolean
  onAsk: (direction: string) => Promise<void>
  onDecide: (asset: string, approve: boolean) => Promise<void>
}) {
  const { t } = useTranslation()
  const [direction, setDirection] = useState('')
  const latest = assets[0]
  const approved = assets.find((a) => a.status === 'approved')
  const waiting = latest?.status === 'drawing' || latest?.status === 'pending'

  return (
    <article className="flex flex-col gap-2 rounded-lg border border-line p-2.5" aria-label={name}>
      <h3 className="text-body font-semibold">{name}</h3>
      {latest?.status === 'pending' ? (
        <>
          <Shown campaignId={campaignId} asset={latest} name={name} />
          {latest.direction && <p className="text-caption text-mute-soft">{latest.direction}</p>}
          <div className="flex gap-1.5">
            <Btn main disabled={disabled} onClick={() => void onDecide(latest.id, true)}>
              {t('gmLive.media.approve')}
            </Btn>
            <Btn disabled={disabled} onClick={() => void onDecide(latest.id, false)}>
              {t('gmLive.media.reject')}
            </Btn>
          </div>
        </>
      ) : (
        approved && <Shown campaignId={campaignId} asset={approved} name={name} />
      )}
      {latest?.status === 'approved' && <p className="text-caption text-stat-hp">{t('prep.media.kept')}</p>}
      {latest?.status === 'drawing' && (
        <p role="status" className="text-caption text-mute-soft">
          {kind === 'intro' ? t('prep.media.filming') : t('gmLive.media.drawing')}
        </p>
      )}
      {latest?.status === 'rejected' && latest.error && (
        <p className="text-caption text-stat-atk">{latest.error}</p>
      )}
      {!latest && <p className="text-caption text-mute-soft">{t('prep.media.none')}</p>}
      {!waiting && (
        <form
          className="flex gap-1.5"
          onSubmit={(e) => {
            e.preventDefault()
            void onAsk(direction.trim()).then(() => setDirection(''))
          }}
        >
          <input
            className={`${field} min-w-0 flex-1`}
            value={direction}
            onChange={(e) => setDirection(e.target.value)}
            maxLength={300}
            placeholder={t('gmLive.media.direction')}
            aria-label={t('gmLive.media.direction')}
          />
          <Btn type="submit" disabled={disabled}>
            {latest ? t('prep.media.redraw') : t('gmLive.media.ask')}
          </Btn>
        </form>
      )}
    </article>
  )
}
