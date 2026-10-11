import { useTranslation } from 'react-i18next'
import { Btn } from '@/features/gm/live/ui'
import type { Hd2dScene } from '@/features/hd2d/build'

/**
 * campaign/watch-prep-in-hd2d — beside the scene in HD-2D: what it still
 * lacks (an element not generated yet, an NPC without a figure), said
 * rather than hidden, and the PixelLab images waiting for the GM, kept
 * or discarded with one tap.
 */
export function Hd2dSection({
  scene,
  busy,
  failed,
  onDecide,
}: {
  scene: Hd2dScene
  busy: boolean
  failed: string | null
  onDecide: (asset: string, keep: boolean) => void
}) {
  const { t } = useTranslation()
  const generations = scene.missing.reduce((n, m) => n + m.generations, 0)
  return (
    <section aria-label={t('atlas.hd2d.title')} className="flex flex-col gap-2">
      <h3 className="type-label text-mute-soft">{t('atlas.hd2d.title')}</h3>
      <p className="text-caption text-mute-soft">{t('atlas.hd2d.hint')}</p>
      {scene.pending.length > 0 && (
        <>
          <h4 className="type-label text-chalk">{t('atlas.hd2d.pending')}</h4>
          <p className="text-caption text-mute-soft">{t('atlas.hd2d.pendingHint')}</p>
          <ul className="flex flex-col gap-1.5">
            {scene.pending.map((p) => (
              <li key={p.asset} className="flex flex-wrap items-center gap-1.5">
                <span className="text-caption text-chalk-soft">{p.name}</span>
                <Btn main disabled={busy} onClick={() => onDecide(p.asset, true)}>
                  {t('atlas.hd2d.keep')}
                </Btn>
                <Btn disabled={busy} onClick={() => onDecide(p.asset, false)}>
                  {t('atlas.hd2d.discard')}
                </Btn>
              </li>
            ))}
          </ul>
        </>
      )}
      <h4 className="type-label text-chalk">{t('atlas.hd2d.missing')}</h4>
      {scene.missing.length === 0 ? (
        <p className="text-caption text-mute-soft">{t('atlas.hd2d.missingNone')}</p>
      ) : (
        <>
          <p className="text-caption text-mute-soft">{t('atlas.hd2d.missingHint', { count: generations })}</p>
          <ul className="flex flex-col gap-1" aria-label={t('atlas.hd2d.missing')}>
            {scene.missing.map((m) => (
              <li key={m.subject} className="text-caption text-stat-hp">
                {m.kind === 'character'
                  ? t('atlas.hd2d.missingCharacter', { name: m.name })
                  : t('atlas.hd2d.missingElement', { name: m.name })}
              </li>
            ))}
          </ul>
        </>
      )}
      {failed && (
        <p role="alert" className="text-caption text-stat-hp">
          {t('atlas.hd2d.failed', { code: failed })}
        </p>
      )}
    </section>
  )
}
