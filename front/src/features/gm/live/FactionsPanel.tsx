import { useTranslation } from 'react-i18next'
import { AffinityGauge } from '@/components/game/AffinityGauge'
import type { LiveScreen, Reveal } from '@/lib/evening'
import { Btn, Panel } from './ui'

/**
 * campaign/track-factions-and-goals: each faction's gauge (−1/+1, its
 * rivals recalled — a gain costs them as much), « Rencontrer » without
 * moving it, and the campaign's goals to tick. Every gesture is a
 * reveal of the live session; the phones see met factions and goals.
 */
export function FactionsPanel({
  screen,
  live,
  onReveal,
}: {
  screen: Pick<LiveScreen, 'factions' | 'goals'>
  live: boolean
  onReveal: (r: Reveal) => void
}) {
  const { t } = useTranslation()
  if (screen.factions.length === 0 && screen.goals.length === 0) return null
  return (
    <Panel title={t('gmLive.factions.title')}>
      {!live && <p className="text-caption text-mute-soft">{t('gmLive.factions.notLive')}</p>}
      {screen.factions.length > 0 && (
        <ul className="flex flex-col gap-2">
          {screen.factions.map((f) => (
            <li key={f.id} className="flex flex-col gap-1">
              <div className="flex items-center gap-2">
                <span className="min-w-0 flex-1 truncate text-body font-bold">{f.name}</span>
                {f.met ? (
                  <span className="text-caption text-mute-soft">{t('gmLive.factions.met')}</span>
                ) : (
                  <Btn disabled={!live} onClick={() => onReveal({ kind: 'faction', faction: f.id, delta: 0 })}>
                    {t('gmLive.factions.meet')}
                  </Btn>
                )}
              </div>
              <div className="flex items-center gap-2">
                <Btn
                  disabled={!live || f.affinity <= f.min}
                  aria-label={t('gmLive.factions.lose', { name: f.name })}
                  onClick={() => onReveal({ kind: 'faction', faction: f.id, delta: -1 })}
                >
                  {t('gmLive.scene.back')}
                </Btn>
                <AffinityGauge value={f.affinity} min={f.min} max={f.max} label={t('gmLive.factions.gauge', { name: f.name })} />
                <Btn
                  disabled={!live || f.affinity >= f.max}
                  aria-label={t('gmLive.factions.win', { name: f.name })}
                  onClick={() => onReveal({ kind: 'faction', faction: f.id, delta: 1 })}
                >
                  {t('gmLive.scene.advance')}
                </Btn>
              </div>
              {f.rivals.length > 0 && (
                <span className="text-caption text-mute-soft">
                  {t('gmLive.factions.rivals', { rivals: f.rivals.join(', ') })}
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
      {screen.goals.length > 0 && (
        <fieldset className="flex flex-col gap-1">
          <legend className="type-label">{t('gmLive.factions.goals')}</legend>
          {screen.goals.map((g) => (
            <label key={g.id} className="flex items-center gap-2 text-body">
              <input
                type="checkbox"
                checked={g.done}
                disabled={!live}
                onChange={(e) => onReveal({ kind: 'goal', goal: g.id, done: e.target.checked })}
              />
              <span className="flex-1">{g.title}</span>
              {g.heldBy && <span className="text-caption text-mute-soft">{g.heldBy}</span>}
            </label>
          ))}
        </fieldset>
      )}
    </Panel>
  )
}
