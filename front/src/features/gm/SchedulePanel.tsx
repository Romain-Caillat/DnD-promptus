import { useCallback, useEffect, useRef, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { ApiError } from '@/lib/api'
import { clock, dayAndTime, toLocalInput } from '@/lib/dates'
import {
  chooseDate,
  dropDate,
  fetchSchedule,
  proposeDate,
  setDiscord,
  type GmDate,
  type GmSchedule,
} from '@/lib/schedule'
import { cn } from '@/lib/utils'

type State = { kind: 'loading' } | { kind: 'error' } | { kind: 'ready'; schedule: GmSchedule }

const LENGTHS = [120, 150, 180, 240]

/** Next Thursday at 20:30, the table's usual evening, as a default. */
function nextEvening(): string {
  const d = new Date()
  d.setDate(d.getDate() + ((4 - d.getDay() + 7) % 7 || 7))
  d.setHours(20, 30, 0, 0)
  return toLocalInput(d)
}

/**
 * The next session's date (session/schedule-sessions, planche « Inviter »,
 * moment 7): the GM proposes dates, sees who can make each, fixes one —
 * the table is told in its Discord channel. The server then reminds the
 * day before and an hour before, and opens the lobby on its own a
 * quarter of an hour before; what was sent, and what failed, is listed
 * under the date. The Discord webhook is a secret: once saved, only its
 * end shows. `refreshKey` moves with the live `session` topic.
 */
export function SchedulePanel({ campaignId, refreshKey }: { campaignId: string; refreshKey: number }) {
  const { t } = useTranslation()
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [when, setWhen] = useState(nextEvening)
  const [minutes, setMinutes] = useState(150)
  const [webhook, setWebhook] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: State
    try {
      next = { kind: 'ready', schedule: await fetchSchedule(campaignId) }
    } catch {
      next = { kind: 'error' }
    }
    if (request !== latest.current) return
    setState((s) => (next.kind === 'error' && s.kind === 'ready' ? s : next))
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  async function act(call: () => Promise<unknown>) {
    setError(null)
    setBusy(true)
    try {
      await call()
      await load()
      return true
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
      return false
    } finally {
      setBusy(false)
    }
  }

  if (state.kind === 'loading') return <p role="status">{t('gm.schedule.loading')}</p>
  if (state.kind === 'error') return <p role="alert">{t('gm.schedule.error')}</p>
  const { schedule } = state
  const chosen = schedule.dates.find((d) => d.status === 'chosen')
  const proposed = schedule.dates.filter((d) => d.status === 'proposed')

  function propose(e: FormEvent) {
    e.preventDefault()
    void act(() => proposeDate(campaignId, new Date(when).toISOString(), minutes))
  }

  return (
    <section className="surface-slab flex flex-col gap-4 p-4" aria-label={t('gm.schedule.title')}>
      <header className="flex items-baseline justify-between gap-2">
        <h2 className="type-title text-heading">{t('gm.schedule.title')}</h2>
        <span className="type-label">{t('gm.schedule.session', { number: schedule.number })}</span>
      </header>
      {error && (
        <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
          {t(`gm.schedule.errors.${error}`, { defaultValue: t('gm.schedule.errors.UNEXPECTED', { code: error }) })}
        </p>
      )}

      {chosen && (
        <div className="flex flex-col gap-1.5 rounded-button bg-ivory px-3.5 py-3 text-ink shadow-ivory-flat">
          <span className="type-label text-ink-soft">{t('gm.schedule.fixed')}</span>
          <b className="type-title text-[16px] first-letter:uppercase">{dayAndTime(chosen.startsAt)}</b>
          <span className="text-caption">{t('gm.schedule.lobbyAt', { time: clock(chosen.lobbyOpensAt) })}</span>
          <span className="text-caption">{t('gm.schedule.reminders')}</span>
          {chosen.log.length > 0 && (
            <ul className="flex flex-col gap-0.5 text-caption" aria-label={t('gm.schedule.sent')}>
              {chosen.log.map((l) => (
                <li key={`${l.kind}-${l.at}`}>
                  {l.ok
                    ? t('gm.schedule.logLine', { what: t(`gm.schedule.log.${l.kind}`), time: clock(l.at) })
                    : t('gm.schedule.logFailed', { what: t(`gm.schedule.log.${l.kind}`), time: clock(l.at), detail: l.detail })}
                </li>
              ))}
            </ul>
          )}
          <Button variant="outline" size="sm" className="self-start" disabled={busy} onClick={() => void act(() => dropDate(campaignId, chosen.id))}>
            {t('gm.schedule.drop')}
          </Button>
        </div>
      )}

      <section className="flex flex-col gap-2" aria-label={t('gm.schedule.proposed')}>
        <h3 className="type-label">{t('gm.schedule.proposed')}</h3>
        {proposed.length === 0 && <p className="text-body text-mute">{t('gm.schedule.none')}</p>}
        <ul className="flex flex-col gap-2">
          {proposed.map((d) => (
            <ProposedDate
              key={d.id}
              date={d}
              players={schedule.players}
              busy={busy}
              onChoose={() => void act(() => chooseDate(campaignId, d.id))}
              onDrop={() => void act(() => dropDate(campaignId, d.id))}
            />
          ))}
        </ul>
        <form className="flex flex-wrap items-end gap-2" onSubmit={propose}>
          <label className="flex flex-col gap-1 text-caption">
            {t('gm.schedule.when')}
            <input
              type="datetime-local"
              required
              className="pixel-field px-2 py-1.5 text-body"
              value={when}
              onChange={(e) => setWhen(e.target.value)}
            />
          </label>
          <label className="flex flex-col gap-1 text-caption">
            {t('gm.schedule.length')}
            <select
              className="pixel-field px-2 py-1.5 text-body"
              value={minutes}
              onChange={(e) => setMinutes(Number(e.target.value))}
            >
              {LENGTHS.map((m) => (
                <option key={m} value={m}>
                  {t('gm.schedule.hours', { count: m / 60 })}
                </option>
              ))}
            </select>
          </label>
          <Button type="submit" disabled={busy}>
            {t('gm.schedule.add')}
          </Button>
        </form>
      </section>

      <section className="flex flex-col gap-2" aria-label={t('gm.schedule.discord')}>
        <h3 className="type-label">{t('gm.schedule.discord')}</h3>
        <p className="text-caption text-mute-soft">{t('gm.schedule.discordHelp')}</p>
        <p className="text-body">
          {schedule.discord.configured
            ? t('gm.schedule.discordSet', { hint: schedule.discord.hint })
            : t('gm.schedule.discordNone')}
        </p>
        <form
          className="flex flex-wrap items-end gap-2"
          onSubmit={(e) => {
            e.preventDefault()
            void act(() => setDiscord(campaignId, webhook)).then((ok) => ok && setWebhook(''))
          }}
        >
          <input
            type="url"
            aria-label={t('gm.schedule.discord')}
            className="min-w-0 flex-1 pixel-field px-2 py-1.5 text-body"
            value={webhook}
            onChange={(e) => setWebhook(e.target.value)}
          />
          <Button type="submit" disabled={busy || !webhook.trim()}>
            {t('gm.schedule.discordSave')}
          </Button>
          {schedule.discord.configured && (
            <Button type="button" variant="outline" disabled={busy} onClick={() => void act(() => setDiscord(campaignId, null))}>
              {t('gm.schedule.discordClear')}
            </Button>
          )}
        </form>
      </section>
    </section>
  )
}

function ProposedDate({
  date,
  players,
  busy,
  onChoose,
  onDrop,
}: {
  date: GmDate
  players: GmSchedule['players']
  busy: boolean
  onChoose: () => void
  onDrop: () => void
}) {
  const { t } = useTranslation()
  const yes = date.answers.filter((a) => a.available).length
  return (
    <li className="flex flex-col gap-1.5 rounded-button border border-line px-3 py-2.5">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <b className="text-body first-letter:uppercase">{dayAndTime(date.startsAt)}</b>
        <span className={cn('text-caption', yes === players.length && players.length > 0 ? 'text-chalk' : 'text-mute-soft')}>
          {t('gm.schedule.yes', { count: yes, total: players.length })}
        </span>
      </div>
      <ul className="flex flex-wrap gap-1.5 text-caption" aria-label={t('gm.schedule.answers')}>
        {players.map((p) => {
          const a = date.answers.find((x) => x.playerId === p.id)
          return (
            <li
              key={p.id}
              className={cn(
                'rounded-full border px-2.5 py-0.5',
                a?.available ? 'border-ivory text-chalk' : 'border-line text-mute-soft',
              )}
            >
              {t('gm.schedule.answerOf', {
                name: p.nickname,
                answer: a
                  ? a.available
                    ? t('gm.schedule.available')
                    : t('gm.schedule.unavailable')
                  : t('gm.schedule.noAnswer'),
              })}
            </li>
          )
        })}
      </ul>
      <div className="flex gap-2">
        <Button size="sm" disabled={busy} onClick={onChoose}>
          {t('gm.schedule.choose')}
        </Button>
        <Button size="sm" variant="outline" disabled={busy} onClick={onDrop}>
          {t('gm.schedule.drop')}
        </Button>
      </div>
    </li>
  )
}
