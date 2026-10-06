import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ApiError } from '@/lib/api'
import { answerDates, calendarUrl, fetchBetween, formatDay, formatWhen, type BetweenView } from '@/lib/between'
import { cn } from '@/lib/utils'

/**
 * Between two sessions (player/play-between-sessions, planche « Entre
 * deux »): what my character gained last time, « Précédemment… » once
 * the GM published it with what we know and what stays open, the
 * chronicle, and the next date — the proposed times I tick, then the
 * chosen one with its calendar reminder. Levelling up lives in the
 * character tab; this screen only points to it.
 */
export function BetweenPanel({
  campaignId,
  refreshKey,
  seated,
}: {
  campaignId: string
  refreshKey: number
  seated: boolean
}) {
  const { t } = useTranslation()
  const [view, setView] = useState<BetweenView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: BetweenView | null
    try {
      next = await fetchBetween(campaignId)
    } catch {
      next = null
    }
    if (request !== latest.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setView((v) => next ?? v)
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  if (!view) return null
  const { lastSession: last, previously, chronicle, next } = view

  async function toggle(at: string) {
    if (!next) return
    const mine = next.mine ?? []
    const available = mine.includes(at) ? mine.filter((x) => x !== at) : [...mine, at]
    setError(null)
    try {
      latest.current++
      setView(await answerDates(campaignId, available))
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    }
  }

  return (
    <div className="flex flex-col gap-3">
      {next && (
        <section className="surface-slab flex flex-col gap-2 p-3.5" aria-label={t('evening.between.next.title', { number: next.number })}>
          <span className="type-label">{t('evening.between.next.title', { number: next.number })}</span>
          {next.chosenAt ? (
            <>
              <p className="type-title text-[20px]">{formatWhen(next.chosenAt)}</p>
              {next.lobbyAt && (
                <p className="text-caption text-chalk-soft">{t('evening.between.next.lobby', { when: formatWhen(next.lobbyAt) })}</p>
              )}
              <a className="self-start text-body underline underline-offset-4" href={calendarUrl(campaignId)} download>
                {t('evening.between.next.remind')}
              </a>
            </>
          ) : (
            <>
              <p className="text-caption text-chalk-soft">
                {seated ? t('evening.between.next.ask') : t('evening.between.next.proposed')}
              </p>
              <div className="flex flex-wrap gap-1.5">
                {next.options.map((o) => {
                  const on = next.mine?.includes(o) ?? false
                  return (
                    <button
                      key={o}
                      type="button"
                      aria-pressed={on}
                      disabled={!seated}
                      className={cn(
                        'rounded-button border px-3 py-2 text-body',
                        on ? 'border-ivory bg-ivory text-ink' : 'border-line',
                      )}
                      onClick={() => void toggle(o)}
                    >
                      {formatWhen(o)}
                    </button>
                  )
                })}
              </div>
              <p className="text-caption text-mute">
                {next.mine === null && seated ? t('evening.between.next.unanswered') : t('evening.between.next.answered', { count: next.answered })}
              </p>
            </>
          )}
          {error && (
            <p role="alert" className="text-caption text-stat-atk">
              {t(`evening.between.errors.${error}`, { defaultValue: t('evening.between.errors.UNEXPECTED') })}
            </p>
          )}
        </section>
      )}
      {last && seated && (last.xpGained !== 0 || last.gains.length > 0) && (
        <section className="flex flex-col gap-1.5">
          <h3 className="type-label">{t('evening.between.gains.title', { number: last.number })}</h3>
          <ul className="flex flex-wrap gap-1.5">
            {last.xpGained !== 0 && (
              <li className="rounded-full bg-ivory px-2.5 py-1 text-caption text-ink">
                {t('evening.between.gains.xp', { count: last.xpGained })}
              </li>
            )}
            {last.gains.map((g) => (
              <li key={`${g.kind}|${g.label}`} className="rounded-full border border-line px-2.5 py-1 text-caption">
                {t('evening.between.gains.item', { label: g.label, delta: g.delta > 0 ? `+${g.delta}` : String(g.delta) })}
              </li>
            ))}
          </ul>
          {last.xpGained > 0 && <p className="text-caption text-mute">{t('evening.between.gains.levelHint')}</p>}
        </section>
      )}
      {last && !last.published && <p className="text-caption text-mute">{t('evening.between.awaitingRecap')}</p>}
      {previously && (
        <section className="surface-slab flex flex-col gap-2 p-3.5">
          <span className="type-label">{t('evening.previously')}</span>
          <p className="type-narration whitespace-pre-line text-[18px] leading-snug">{previously.text}</p>
          <Facts title={t('evening.between.clues')} items={[...previously.clues, ...previously.revelations]} />
          <Facts title={t('evening.between.open')} items={previously.openThreads} />
        </section>
      )}
      {chronicle.length > 0 && (
        <section className="flex flex-col gap-1.5">
          <h3 className="type-label">{t('evening.between.chronicle')}</h3>
          <ol aria-label={t('evening.between.chronicle')} className="flex flex-col gap-2 border-l border-line pl-3">
            {chronicle.map((e) => (
              <li key={e.number} className="flex flex-col">
                <span className="text-caption text-mute-soft">
                  {t('evening.between.entry', { number: e.number })}
                  {e.date && ` · ${formatDay(e.date)}`}
                </span>
                {e.title && <b className="text-body">{e.title}</b>}
                {e.text && <span className="text-caption text-chalk-soft">{e.text}</span>}
              </li>
            ))}
          </ol>
        </section>
      )}
    </div>
  )
}

function Facts({ title, items }: { title: string; items: string[] }) {
  if (items.length === 0) return null
  return (
    <div className="flex flex-col gap-1">
      <span className="text-caption text-mute-soft">{title}</span>
      <ul className="flex flex-col gap-1">
        {items.map((x) => (
          <li key={x} className="rounded-button bg-ivory px-3 py-1.5 text-caption text-ink">
            {x}
          </li>
        ))}
      </ul>
    </div>
  )
}
