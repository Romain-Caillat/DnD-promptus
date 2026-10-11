import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Btn, field } from '@/features/gm/live/ui'
import type { Atlas, AtlasMap, Cast } from '@/lib/maps'
import type { Picked, Target } from './cast'

type Kind = 'npc' | 'clue' | 'fight'
const KINDS: Kind[] = ['npc', 'clue', 'fight']

/**
 * The side panel's cast (campaign/place-cast-and-clues): the NPC, clue
 * or fight chosen on the map — moved or taken off, a fight showing how
 * many opponents its battle plan already places and opening it — and
 * the tool that sets one down on the next cell tapped, an NPC for every
 * scene on the map or for one.
 */
export function CastSection({
  cast,
  castable,
  picked,
  placing,
  busy,
  maps,
  sceneLink,
  onPlace,
  onCancel,
  onRemove,
  onOpenMap,
}: {
  cast: Cast
  castable: Atlas['castable']
  picked: Picked | null
  placing: Target | null
  busy: boolean
  maps: Map<string, AtlasMap>
  sceneLink: (id: string) => string
  onPlace: (target: Target) => void
  onCancel: () => void
  onRemove: (p: Picked) => void
  onOpenMap: (map: string) => void
}) {
  const { t } = useTranslation()
  const [kind, setKind] = useState<Kind>('npc')
  const [choice, setChoice] = useState('')
  const [scene, setScene] = useState('')
  const link = 'text-caption text-chalk underline underline-offset-4'
  const sceneTitle = (id: string) => {
    const s = castable.scenes.find((x) => x.id === id)
    return s ? t('atlas.pins.scene', { number: s.number, title: s.title }) : id
  }
  const nameOf = (target: Target): string => {
    if (target.kind === 'npc') return castable.npcs.find((n) => n.id === target.id)?.name ?? target.id
    if (target.kind === 'clue') return castable.clues.find((c) => c.id === target.id)?.text ?? target.id
    if (target.kind === 'fight') return castable.fights.find((f) => f.scene === target.id)?.title ?? target.id
    return target.id
  }

  const chosen = (() => {
    if (!picked) return null
    const actions = (move: Target) => (
      <div className="flex flex-wrap gap-1.5">
        <Btn disabled={busy} onClick={() => onPlace(move)}>
          {t('atlas.cast.move')}
        </Btn>
        <Btn disabled={busy} onClick={() => onRemove(picked)}>
          {t('atlas.cast.remove')}
        </Btn>
      </div>
    )
    if (picked.kind === 'npc') {
      const m = cast.npcs.find((n) => n.npc === picked.id && n.index === picked.index)
      if (!m) return null
      const move: Target = { kind: picked.kind, id: m.npc, index: m.index }
      return (
        <section aria-label={m.name} className="flex flex-col gap-1.5">
          <h3 className="type-label text-mute-soft">{t('atlas.cast.npcChosen', { name: m.name })}</h3>
          <p className="text-caption text-chalk-soft">
            {m.scene ? t('atlas.cast.forScene', { scene: sceneTitle(m.scene) }) : t('atlas.cast.everyScene')}
          </p>
          {actions(move)}
        </section>
      )
    }
    if (picked.kind === 'clue') {
      const m = cast.clues.find((c) => c.clue === picked.id)
      if (!m) return null
      const move: Target = { kind: picked.kind, id: m.clue }
      return (
        <section aria-label={t('atlas.cast.clueChosen')} className="flex flex-col gap-1.5">
          <h3 className="type-label text-mute-soft">{t('atlas.cast.clueChosen')}</h3>
          <p className="text-body text-chalk-soft">{m.text}</p>
          <Link className={link} to={sceneLink(m.scene)}>
            {sceneTitle(m.scene)}
          </Link>
          {actions(move)}
        </section>
      )
    }
    const m = cast.fights.find((f) => f.scene === picked.id)
    if (!m) return null
    const move: Target = { kind: picked.kind, id: m.scene }
    return (
      <section aria-label={t('atlas.cast.fightChosen', { title: m.title })} className="flex flex-col gap-1.5">
        <h3 className="type-label text-mute-soft">{t('atlas.cast.fightChosen', { title: m.title })}</h3>
        <Link className={link} to={sceneLink(m.scene)}>
          {t('atlas.pins.scene', { number: m.number, title: m.title })}
        </Link>
        {!m.map ? (
          <p className="text-caption text-stat-hp">{t('atlas.cast.noPlan')}</p>
        ) : m.placed === undefined ? (
          <p className="text-caption text-mute-soft">{t('atlas.cast.planUnknown')}</p>
        ) : (
          <p className={m.placed < m.opponents ? 'text-caption text-stat-hp' : 'text-caption text-chalk-soft'}>
            {t('atlas.cast.placed', { placed: m.placed, total: m.opponents })}
          </p>
        )}
        {m.map && maps.has(m.map) && <Btn onClick={() => onOpenMap(m.map!)}>{t('atlas.cast.openPlan')}</Btn>}
        {actions(move)}
      </section>
    )
  })()

  const options: { id: string; name: string }[] =
    kind === 'npc'
      ? castable.npcs.map((n) => ({ id: n.id, name: n.name }))
      : kind === 'clue'
        ? castable.clues.map((c) => ({ id: c.id, name: c.anchor ? t('atlas.cast.alreadyPlaced', { name: c.text }) : c.text }))
        : castable.fights.map((f) => ({
            id: f.scene,
            name: f.anchor
              ? t('atlas.cast.alreadyPlaced', { name: t('atlas.pins.scene', { number: f.number, title: f.title }) })
              : t('atlas.pins.scene', { number: f.number, title: f.title }),
          }))
  const target = (): Target =>
    kind === 'npc' ? { kind, id: choice, ...(scene ? { scene } : {}) } : { kind, id: choice }

  return (
    <>
      {chosen}
      <h3 className="type-label text-mute-soft">{t('atlas.cast.tool')}</h3>
      {placing && placing.kind !== 'place' ? (
        <div className="flex flex-wrap items-center gap-1.5" role="status">
          <p className="text-caption text-chalk-soft">{t('atlas.cast.tapCell', { name: nameOf(placing) })}</p>
          <Btn onClick={onCancel}>{t('atlas.pins.cancel')}</Btn>
        </div>
      ) : (
        <div className="flex flex-col gap-1.5">
          <div className="flex flex-wrap gap-1.5" role="group" aria-label={t('atlas.cast.kind')}>
            {KINDS.map((k) => (
              <button
                key={k}
                type="button"
                className="pixel-choice py-1 pr-2 pb-1.5 text-caption font-semibold"
                aria-pressed={kind === k}
                onClick={() => {
                  setKind(k)
                  setChoice('')
                }}
              >
                {t(`atlas.cast.kinds.${k}`)}
              </button>
            ))}
          </div>
          {options.length === 0 ? (
            <p className="text-caption text-mute-soft">{t(`atlas.cast.none.${kind}`)}</p>
          ) : (
            <>
              <select
                className={field}
                value={choice}
                onChange={(e) => setChoice(e.target.value)}
                aria-label={t(`atlas.cast.choose.${kind}`)}
              >
                <option value="">{t(`atlas.cast.choose.${kind}`)}</option>
                {options.map((o) => (
                  <option key={o.id} value={o.id}>
                    {o.name}
                  </option>
                ))}
              </select>
              {kind === 'npc' && (
                <select className={field} value={scene} onChange={(e) => setScene(e.target.value)} aria-label={t('atlas.cast.scene')}>
                  <option value="">{t('atlas.cast.everyScene')}</option>
                  {castable.scenes.map((s) => (
                    <option key={s.id} value={s.id}>
                      {t('atlas.pins.scene', { number: s.number, title: s.title })}
                    </option>
                  ))}
                </select>
              )}
              <Btn main disabled={!choice || busy} onClick={() => onPlace(target())}>
                {t('atlas.cast.placeHere')}
              </Btn>
            </>
          )}
        </div>
      )}
    </>
  )
}
