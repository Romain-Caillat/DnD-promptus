import { useTranslation } from 'react-i18next'
import { AffinityGauge } from '@/components/game/AffinityGauge'
import type { GoalStatus, LiveScreen, Reveal } from '@/lib/evening'
import { Btn, Panel } from './ui'

/**
 * Factions and campaign goals (campaign/track-factions-and-goals): each
 * faction's gauge with one-tap −1 / +1 — a gain costs its rivals, the
 * server says how much — and « Faire connaître » for one the table has
 * not heard of; each goal hidden, known or reached. Every gesture is a
 * reveal of the live session, written in the journal.
 */
export function FactionsPanel({ screen, onReveal }: { screen: LiveScreen; onReveal: (r: Reveal) => void }) {
  const { t } = useTranslation()
  if (screen.factions.length === 0 && screen.goals.length === 0) return null
  const live = screen.session?.status === 'live'
  const nameOf = (id: string) => screen.factions.find((f) => f.id === id)?.name ?? id
  const setGoal = (goal: string, status: GoalStatus | null) => onReveal({ kind: 'goal', goal, status })
  return (
    <Panel title={t('gmLive.factions.title')}>
      {screen.factions.length > 0 && (
        <ul className="flex flex-col gap-2">
          {screen.factions.map((f) => (
            <li key={f.id} className="flex flex-col gap-1 rounded-lg border border-line px-2.5 py-2">
              <div className="flex items-center justify-between gap-2">
                <span className="text-body font-bold">{f.name}</span>
                <span className="text-caption text-mute-soft">
                  {f.known ? t('gmLive.factions.known') : t('gmLive.factions.unknown')}
                </span>
              </div>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <AffinityGauge name={f.name} value={f.affinity} min={f.min} max={f.max} cell={10} />
                {live && (
                  <span className="flex gap-1">
                    <Btn
                      aria-label={t('gmLive.factions.down', { name: f.name })}
                      disabled={f.affinity <= f.min}
                      onClick={() => onReveal({ kind: 'affinity', faction: f.id, delta: -1 })}
                    >
                      {t('gmLive.factions.minus')}
                    </Btn>
                    <Btn
                      aria-label={t('gmLive.factions.up', { name: f.name })}
                      disabled={f.affinity >= f.max}
                      onClick={() => onReveal({ kind: 'affinity', faction: f.id, delta: 1 })}
                    >
                      {t('gmLive.factions.plus')}
                    </Btn>
                    {!f.known && (
                      <Btn onClick={() => onReveal({ kind: 'faction', faction: f.id })}>{t('gmLive.factions.meet')}</Btn>
                    )}
                  </span>
                )}
              </div>
              {f.rivals.length > 0 && (
                <span className="text-caption text-mute-soft">
                  {t('gmLive.factions.rivals', { names: f.rivals.map(nameOf).join(', ') })}
                </span>
              )}
              {f.diplomacy && <span className="text-caption text-mute">{f.diplomacy}</span>}
            </li>
          ))}
        </ul>
      )}
      {screen.goals.length > 0 && (
        <>
          <h3 className="type-label text-mute-soft">{t('gmLive.factions.goals')}</h3>
          <ul className="flex flex-col gap-1.5">
            {screen.goals.map((g) => (
              <li key={g.id} className="flex flex-wrap items-center justify-between gap-2">
                <span className={g.status === 'done' ? 'text-body line-through' : 'text-body'}>
                  {g.title}
                  {g.heldBy && <span className="text-caption text-mute-soft"> · {nameOf(g.heldBy)}</span>}
                </span>
                {live && (
                  <span className="flex gap-1">
                    <Btn main={g.status === null} onClick={() => setGoal(g.id, null)} aria-pressed={g.status === null}>
                      {t('gmLive.factions.goal.hidden')}
                    </Btn>
                    <Btn main={g.status === 'known'} onClick={() => setGoal(g.id, 'known')} aria-pressed={g.status === 'known'}>
                      {t('gmLive.factions.goal.known')}
                    </Btn>
                    <Btn main={g.status === 'done'} onClick={() => setGoal(g.id, 'done')} aria-pressed={g.status === 'done'}>
                      {t('gmLive.factions.goal.done')}
                    </Btn>
                  </span>
                )}
              </li>
            ))}
          </ul>
        </>
      )}
    </Panel>
  )
}
