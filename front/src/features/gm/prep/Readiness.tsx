import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Panel } from '@/features/gm/live/ui'
import { fetchReadiness, type ActReadiness, type ReadinessGap } from '@/lib/prep'
import { cn } from '@/lib/utils'

/**
 * The readiness of each act, fetched again whenever the campaign
 * changes (`version`): `null` while the first answer is on its way.
 */
export function useReadiness(campaignId: string, version: string): ActReadiness[] | null {
  const [acts, setActs] = useState<ActReadiness[] | null>(null)
  const latest = useRef(0)
  useEffect(() => {
    const request = ++latest.current
    fetchReadiness(campaignId).then(
      (list) => {
        if (request === latest.current) setActs(list)
      },
      () => {
        // Kept as it was; the next change tries again.
      },
    )
  }, [campaignId, version])
  return acts
}

/** A bar, filled to `done` out of `total`. */
export function Gauge({ done, total, className }: { done: number; total: number; className?: string }) {
  const { t } = useTranslation()
  const pct = total === 0 ? 0 : Math.round((done / total) * 100)
  return (
    <div
      role="meter"
      aria-valuemin={0}
      aria-valuemax={total}
      aria-valuenow={done}
      aria-label={t('prep.readiness.meter', { done, total })}
      className={cn('h-1.5 overflow-hidden rounded-sm bg-surface', className)}
    >
      <i className="block h-full bg-chalk" style={{ width: `${pct}%` }} />
    </div>
  )
}

function GapText({ gap }: { gap: ReadinessGap }) {
  const { t } = useTranslation()
  const fields = (gap.fields ?? []).map((f) => t(`prep.readiness.fields.${f}`, { defaultValue: f })).join(', ')
  return (
    <li className="flex flex-col text-caption">
      <span className="text-chalk">
        {t(`prep.readiness.gap.${gap.code}`, {
          defaultValue: gap.detail,
          name: gap.name || gap.id,
          fields,
          count: gap.paths ?? 0,
        })}
      </span>
      {(gap.code === 'NOT_STAGED' || gap.code === 'SIMULATION_FAILED') && (
        <span className="font-mono text-[11px] text-mute-soft">{gap.detail}</span>
      )}
    </li>
  )
}

/** Every act's gauge, and what it lacks; the GM decides. */
export function Readiness({ acts }: { acts: ActReadiness[] | null }) {
  const { t } = useTranslation()
  return (
    <Panel title={t('prep.readiness.title')}>
      <p className="text-caption text-mute-soft">{t('prep.readiness.hint')}</p>
      {acts === null && <p role="status">{t('prep.loading')}</p>}
      {acts?.map((a) => (
        <details key={a.act} className="flex flex-col gap-2 rounded-xl border border-line p-2.5" open={!a.ready}>
          <summary className="flex cursor-pointer flex-col gap-1.5">
            <span className="flex items-baseline justify-between gap-2">
              <b className="text-body">{a.title}</b>
              <span className={cn('text-caption font-semibold', a.ready ? 'text-chalk' : 'text-stat-init')}>
                {a.ready ? t('prep.readiness.ready') : t('prep.readiness.notReady', { done: a.done, total: a.total })}
              </span>
            </span>
            <Gauge done={a.done} total={a.total} />
          </summary>
          <ul className="mt-2 flex flex-col gap-2">
            {a.checks
              .filter((c) => c.total > 0)
              .map((c) => (
                <li key={c.kind} className="flex flex-col gap-1">
                  <span className="flex justify-between text-caption text-mute-soft">
                    <span>{t(`prep.readiness.check.${c.kind}`)}</span>
                    <span>
                      {c.done} / {c.total}
                    </span>
                  </span>
                  {c.gaps.length > 0 && (
                    <ul className="flex flex-col gap-1 pl-2">
                      {c.gaps.map((g) => (
                        <GapText key={`${g.code}${g.id}`} gap={g} />
                      ))}
                    </ul>
                  )}
                </li>
              ))}
          </ul>
        </details>
      ))}
    </Panel>
  )
}
