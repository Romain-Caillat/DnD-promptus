import { useTranslation } from 'react-i18next'
import type { AtlasMap, Frieze, FriezeStop } from '@/lib/maps'

/** A stop of the frieze by its place: act, slot, stop. */
export interface FriezeAt {
  act: string
  slot: number
  stop: number
}

/**
 * The stop marked as the map shown: the one tapped when it still shows
 * that map, else the first stop of the frieze on it.
 */
export function markedStop(frieze: Frieze, shown: string | null, tapped: FriezeAt | null): FriezeAt | null {
  if (!shown) return null
  const at = (p: FriezeAt) => frieze.acts.find((a) => a.act === p.act)?.slots[p.slot]?.stops[p.stop]
  if (tapped && at(tapped)?.map === shown) return tapped
  for (const a of frieze.acts) {
    for (const [slot, s] of a.slots.entries()) {
      const stop = s.stops.findIndex((st) => st.map === shown)
      if (stop >= 0) return { act: a.act, slot, stop }
    }
  }
  return null
}

/**
 * The story frieze (campaign/order-maps-by-story), at the foot of the
 * Atlas in « Vue MJ »: one row per evening — an act — with the maps its
 * scenes cross in the order of the story, as the server computes it.
 * A branch's maps are stacked in one slot; a place whose map is still to
 * create shows dashed, and tapping it leads to making that map. An act
 * with no scene offers to write it. The maps no scene crosses follow,
 * « Hors de l'histoire ». Scrolls sideways on a phone or a tablet.
 */
export function StoryFrieze({
  frieze,
  maps,
  marked,
  shown,
  onMap,
  onCreate,
  onWrite,
}: {
  frieze: Frieze
  maps: Map<string, AtlasMap>
  marked: FriezeAt | null
  /** The map shown, marked among the maps outside the story. */
  shown: string | null
  /** A map tapped: from a stop of an act, or from outside the story (`null`). */
  onMap: (map: string, at: FriezeAt | null) => void
  onCreate: (place: string) => void
  onWrite: () => void
}) {
  const { t } = useTranslation()
  const scenesOf = (s: FriezeStop) => s.scenes.map((x) => t('atlas.pins.scene', { number: x.number, title: x.title })).join(', ')
  const isMarked = (at: FriezeAt) =>
    marked !== null && marked.act === at.act && marked.slot === at.slot && marked.stop === at.stop
  return (
    <nav aria-label={t('atlas.frieze.label')} className="flex gap-4 overflow-x-auto pb-1">
      {frieze.acts.map((a) => {
        const title = t('atlas.frieze.evening', { number: a.evening, title: a.title })
        return (
          <section key={a.act} aria-label={title} className="flex shrink-0 flex-col gap-1.5">
            <h3 className="type-label text-mute-soft">{title}</h3>
            <ol className="flex items-start gap-1.5">
              {a.slots.length === 0 && (
                <li>
                  <button
                    type="button"
                    className="pixel-choice border border-dashed border-line py-1.5 pr-2.5 pb-2 text-caption font-semibold text-mute-soft"
                    onClick={onWrite}
                  >
                    {t('atlas.frieze.write')}
                  </button>
                </li>
              )}
              {a.slots.map((s, slot) => (
                <li key={slot} className="flex items-start gap-1.5">
                  {slot > 0 && (
                    <span className="py-1.5 text-caption text-mute-soft" aria-hidden="true">
                      →
                    </span>
                  )}
                  <div
                    className="flex flex-col gap-1"
                    role={s.stops.length > 1 ? 'group' : undefined}
                    aria-label={s.stops.length > 1 ? t('atlas.frieze.branch') : undefined}
                  >
                    {s.stops.map((st, stop) => {
                      const at = { act: a.act, slot, stop }
                      if (!st.map || !maps.has(st.map)) {
                        return (
                          <button
                            key={st.place ?? st.name}
                            type="button"
                            title={scenesOf(st)}
                            className="pixel-choice border border-dashed border-line py-1.5 pr-2.5 pb-2 text-left text-caption font-semibold text-mute-soft"
                            onClick={() => st.place && onCreate(st.place)}
                          >
                            {t('atlas.frieze.missing', { name: st.name })}
                          </button>
                        )
                      }
                      const map = st.map
                      const entry = maps.get(map)!
                      return (
                        <button
                          key={map}
                          type="button"
                          title={scenesOf(st)}
                          className="pixel-choice flex items-center gap-1.5 py-1.5 pr-2.5 pb-2 text-left text-caption font-semibold"
                          aria-current={isMarked(at) ? 'page' : undefined}
                          onClick={() => onMap(map, at)}
                        >
                          <span className="text-mute-soft">{t(`atlas.scaleShort.${entry.map.scale ?? 'encounter'}`)}</span>
                          {st.name}
                        </button>
                      )
                    })}
                  </div>
                </li>
              ))}
            </ol>
          </section>
        )
      })}
      {frieze.outside.length > 0 && (
        <section aria-label={t('atlas.frieze.outside')} className="flex shrink-0 flex-col gap-1.5">
          <h3 className="type-label text-mute-soft">{t('atlas.frieze.outside')}</h3>
          <ul className="flex items-start gap-1.5">
            {frieze.outside.flatMap((id) => {
              const m = maps.get(id)
              if (!m) return []
              return (
                <li key={id}>
                  <button
                    type="button"
                    className="pixel-choice flex items-center gap-1.5 py-1.5 pr-2.5 pb-2 text-caption font-semibold"
                    aria-current={id === shown ? 'page' : undefined}
                    onClick={() => onMap(id, null)}
                  >
                    <span className="text-mute-soft">{t(`atlas.scaleShort.${m.map.scale ?? 'encounter'}`)}</span>
                    {m.map.name}
                  </button>
                </li>
              )
            })}
          </ul>
        </section>
      )}
    </nav>
  )
}
