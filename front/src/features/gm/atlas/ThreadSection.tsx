import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { Atlas, ThreadStop } from '@/lib/maps'

/**
 * The scenes beside the map (campaign/draw-story-thread), in campaign
 * order, each a start or an end of its act or neither; the act whose
 * thread the map draws. A scene tapped is shown on the map (its pin
 * chosen, its segments lit, the map brought to it); a pin tapped on the
 * map marks its scenes here and brings the first into view.
 */
export function ThreadSection({
  stops,
  acts,
  act,
  scene,
  place,
  sceneTitle,
  onAct,
  onScene,
}: {
  /** The map's scenes, in the act chosen. */
  stops: readonly ThreadStop[]
  /** The acts with a scene on this map. */
  acts: Atlas['acts']
  /** `null`: every act. */
  act: string | null
  /** The scene chosen in this list. */
  scene: string | null
  /** The pin chosen on the map. */
  place: string | null
  sceneTitle: (id: string) => string
  onAct: (act: string | null) => void
  onScene: (stop: ThreadStop) => void
}) {
  const { t } = useTranslation()
  const list = useRef<HTMLOListElement>(null)
  const marked = (s: ThreadStop) => (scene ? s.scene === scene : s.place === place)
  const first = stops.find(marked)?.scene ?? null

  useEffect(() => {
    if (!first || !list.current) return
    const el = list.current.querySelector<HTMLElement>(`[data-scene="${CSS.escape(first)}"]`)
    // jsdom has no scrollIntoView.
    el?.scrollIntoView?.({ block: 'nearest' })
  }, [first])

  return (
    <section aria-label={t('atlas.thread.title')} className="flex flex-col gap-1.5">
      <h3 className="type-label text-mute-soft">{t('atlas.thread.title')}</h3>
      {acts.length > 1 && (
        <div className="flex flex-wrap gap-1.5" role="group" aria-label={t('atlas.thread.act')}>
          {[null, ...acts.map((a) => a.id)].map((id) => (
            <button
              key={id ?? ''}
              type="button"
              className="pixel-choice py-1 pr-2 pb-1.5 text-caption font-semibold"
              aria-pressed={act === id}
              onClick={() => onAct(id)}
            >
              {id === null ? t('atlas.thread.allActs') : acts.find((a) => a.id === id)?.title}
            </button>
          ))}
        </div>
      )}
      {stops.length === 0 ? (
        <p className="text-caption text-mute-soft">{t('atlas.thread.none')}</p>
      ) : (
        <ol ref={list} className="flex max-h-64 flex-col gap-1 overflow-y-auto">
          {stops.map((s) => (
            <li key={s.scene}>
              <button
                type="button"
                data-scene={s.scene}
                className="pixel-choice flex w-full flex-wrap items-center gap-1.5 py-1 pr-2 pb-1.5 text-left text-caption"
                aria-current={marked(s) ? 'true' : undefined}
                onClick={() => onScene(s)}
              >
                <span className="font-semibold">{sceneTitle(s.scene)}</span>
                {s.start && <span className="text-mute-soft">· {t('atlas.thread.start')}</span>}
                {s.end && <span className="text-mute-soft">· {t('atlas.thread.end')}</span>}
              </button>
            </li>
          ))}
        </ol>
      )}
    </section>
  )
}
