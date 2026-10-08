import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ApiError } from '@/lib/api'
import { clock, dayAndTime } from '@/lib/dates'
import { answerDate, calendarUrl, fetchPlayerSchedule, type PlayerSchedule } from '@/lib/schedule'
import { cn } from '@/lib/utils'

/**
 * The next session on the phone between two evenings
 * (session/schedule-sessions, planche « Entre deux », moment 6): the
 * date the GM fixed, when the lobby opens, and the way to put it in the
 * phone's own calendar, with its reminders; or the dates the GM proposes,
 * answered with a touch each. The lobby itself shows in this tab as soon
 * as it opens. `refreshKey` moves with the live `session` topic.
 */
export function NextSession({ campaignId, refreshKey }: { campaignId: string; refreshKey: number }) {
  const { t } = useTranslation()
  const [schedule, setSchedule] = useState<PlayerSchedule | null>(null)
  const [error, setError] = useState<string | null>(null)
  /** When the schedule was read: how far the session is, without reading the clock while drawing. */
  const [readAt, setReadAt] = useState(0)
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: PlayerSchedule | null
    try {
      next = await fetchPlayerSchedule(campaignId)
    } catch {
      // Kept as it was; the next change tries again.
      next = null
    }
    if (request !== latest.current || !next) return
    setSchedule(next)
    setReadAt(Date.now())
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  if (!schedule) return null
  const { next } = schedule
  const soon = next && Date.parse(next.startsAt) - readAt < 3_600_000

  return (
    <>
      {next && (
        <div className="flex flex-col gap-1 rounded-button bg-ivory px-3.5 py-3 text-ink shadow-ivory-flat">
          <span className="type-label text-ink-soft">
            {soon ? t('evening.schedule.soon') : t('evening.schedule.next', { number: schedule.number })}
          </span>
          <b className="type-title text-[16px] first-letter:uppercase">{dayAndTime(next.startsAt)}</b>
          <span className="text-caption">{t('evening.schedule.lobbyAt', { time: clock(next.lobbyOpensAt) })}</span>
          <a
            className="self-start text-caption font-bold underline underline-offset-4"
            href={calendarUrl(campaignId)}
            download
          >
            {t('evening.schedule.calendar')}
          </a>
        </div>
      )}
      {schedule.proposed.length > 0 && (
        <section
          className="flex flex-col gap-2"
          aria-label={t(schedule.canAnswer ? 'evening.schedule.ask' : 'evening.schedule.askSpectator', {
            number: next ? schedule.number + 1 : schedule.number,
          })}
        >
          <h2 className="type-title text-[16px]">
            {t(schedule.canAnswer ? 'evening.schedule.ask' : 'evening.schedule.askSpectator', {
              number: next ? schedule.number + 1 : schedule.number,
            })}
          </h2>
          {error && (
            <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
              {t(`evening.errors.${error}`, { defaultValue: t('evening.errors.UNEXPECTED') })}
            </p>
          )}
          <ul className="flex flex-col gap-2">
            {schedule.proposed.map((d) => (
              <li key={d.id} className="flex flex-col gap-1.5 rounded-button border border-line px-3 py-2.5">
                <b className="text-body first-letter:uppercase">{dayAndTime(d.startsAt)}</b>
                <span className="text-caption text-mute-soft">
                  {d.yes.length > 0 ? t('evening.schedule.who', { names: d.yes.join(', ') }) : t('evening.schedule.nobody')}
                </span>
                {schedule.canAnswer && (
                  <div className="grid grid-cols-2 gap-1.5">
                    {[true, false].map((yes) => (
                      <button
                        key={String(yes)}
                        type="button"
                        aria-pressed={d.mine === yes}
                        className={cn(
                          'pixel-choice py-2.5 pr-3 pb-3 text-label type-key',
                        )}
                        onClick={async () => {
                          setError(null)
                          try {
                            setSchedule(await answerDate(campaignId, d.id, yes))
                          } catch (err) {
                            setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
                          }
                        }}
                      >
                        {yes ? t('evening.schedule.yes') : t('evening.schedule.no')}
                      </button>
                    ))}
                  </div>
                )}
              </li>
            ))}
          </ul>
          <p className="text-caption text-mute">{t('evening.schedule.gmPicks')}</p>
        </section>
      )}
    </>
  )
}
