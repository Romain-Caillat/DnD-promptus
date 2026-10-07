import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { formatWhen, type Plan } from '@/lib/between'
import type { LiveScreen, RecapText, SessionInfo } from '@/lib/evening'
import { Btn, Panel, field } from './ui'

/**
 * The recap the morning after (session/write-recaps): ending the evening
 * drafted it from what happened; the GM rereads « Précédemment… », the
 * chronicle's title and two lines, and their own recap — the co-GM may
 * redraft all four — then publishes. Nothing reaches the players before.
 * Keyed by session: the fields start from what the server holds.
 */
export function RecapPanel({
  session,
  aiConfigured,
  onSave,
  onPublish,
  onDraft,
}: {
  session: SessionInfo
  aiConfigured: boolean
  onSave: (text: RecapText) => Promise<void>
  onPublish: (text: RecapText) => Promise<void>
  onDraft: () => Promise<RecapText | null>
}) {
  const { t } = useTranslation()
  const [text, setText] = useState<RecapText>({
    recap: session.recap,
    previously: session.previously,
    title: session.title,
    chronicle: session.chronicle,
  })
  const [busy, setBusy] = useState(false)
  const [saved, setSaved] = useState(false)

  const set = (k: keyof RecapText) => (e: { target: { value: string } }) => {
    setText((x) => ({ ...x, [k]: e.target.value }))
    setSaved(false)
  }
  async function run(task: () => Promise<void>) {
    setBusy(true)
    try {
      await task()
    } finally {
      setBusy(false)
    }
  }

  return (
    <Panel
      title={t('gmLive.recap.title', { number: session.number })}
      actions={
        <span className="text-caption text-mute-soft">
          {session.published ? t('gmLive.recap.published') : t('gmLive.recap.draftState')}
        </span>
      }
    >
      <label className="flex flex-col gap-1 text-caption">
        {t('gmLive.end.previously')}
        <textarea className={field} rows={5} value={text.previously} onChange={set('previously')} />
      </label>
      <label className="flex flex-col gap-1 text-caption">
        {t('gmLive.recap.entryTitle')}
        <input className={field} value={text.title} onChange={set('title')} />
      </label>
      <label className="flex flex-col gap-1 text-caption">
        {t('gmLive.recap.entry')}
        <textarea className={field} rows={2} value={text.chronicle} onChange={set('chronicle')} />
      </label>
      <label className="flex flex-col gap-1 text-caption">
        {t('gmLive.end.recap')}
        <textarea className={field} rows={4} value={text.recap} onChange={set('recap')} />
      </label>
      <div className="flex flex-wrap gap-1.5">
        <Btn
          disabled={busy || !aiConfigured}
          onClick={() =>
            void run(async () => {
              const d = await onDraft()
              if (d) {
                setText(d)
                setSaved(false)
              }
            })
          }
        >
          {t('gmLive.end.draft')}
        </Btn>
        <Btn
          disabled={busy}
          onClick={() =>
            void run(async () => {
              await onSave(text)
              setSaved(true)
            })
          }
        >
          {saved ? t('gmLive.recap.saved') : t('gmLive.recap.save')}
        </Btn>
        <Btn main disabled={busy || !text.previously.trim()} onClick={() => void run(() => onPublish(text))}>
          {session.published ? t('gmLive.recap.republish') : t('gmLive.recap.publish')}
        </Btn>
      </div>
    </Panel>
  )
}

/** A `datetime-local` value as an ISO instant, or `null` when unset. */
function toIso(local: string): string | null {
  if (!local) return null
  const d = new Date(local)
  return Number.isNaN(d.getTime()) ? null : d.toISOString()
}

/**
 * The next date (session/schedule-sessions): the GM proposes a few
 * start times, sees who can, and fixes one; the lobby then opens by
 * itself before it.
 */
export function PlanPanel({
  plan,
  screen,
  onPropose,
  onChoose,
}: {
  plan: Plan | null
  screen: LiveScreen
  onPropose: (options: string[], lobbyMinutes: number) => Promise<void>
  onChoose: (at: string | null) => Promise<void>
}) {
  const { t } = useTranslation()
  const [drafts, setDrafts] = useState<string[]>([''])
  const [lobby, setLobby] = useState(plan?.lobbyMinutes ?? 15)
  const names = new Map(screen.lobby.map((s) => [s.playerId, s.characterName || s.nickname]))
  const players = screen.lobby.filter((s) => s.role === 'player')
  const options = drafts.map(toIso).filter((x): x is string => x !== null)

  return (
    <Panel title={t('gmLive.plan.title')}>
      {plan && plan.options.length > 0 && (
        <ul className="flex flex-col gap-1.5">
          {plan.options.map((o) => {
            const who = plan.answers.filter((a) => a.available.includes(o)).map((a) => names.get(a.playerId) ?? '?')
            const chosen = plan.chosenAt === o
            return (
              <li key={o} className="flex items-center justify-between gap-2 text-caption">
                <span className="flex flex-col">
                  <b className="text-chalk">{formatWhen(o)}</b>
                  <span className="text-mute-soft">
                    {t('gmLive.plan.can', { count: who.length, total: players.length })}
                    {who.length > 0 && ` · ${who.join(', ')}`}
                  </span>
                </span>
                <Btn main={!chosen} onClick={() => void onChoose(chosen ? null : o)}>
                  {chosen ? t('gmLive.plan.unchoose') : t('gmLive.plan.choose')}
                </Btn>
              </li>
            )
          })}
        </ul>
      )}
      {plan?.chosenAt && (
        <p className="text-caption text-chalk-soft">
          {t('gmLive.plan.chosen', { when: formatWhen(plan.chosenAt), minutes: plan.lobbyMinutes })}
        </p>
      )}
      <fieldset className="flex flex-col gap-1.5">
        <legend className="text-caption text-mute-soft">{t('gmLive.plan.propose')}</legend>
        {drafts.map((d, i) => (
          <input
            key={i}
            type="datetime-local"
            aria-label={t('gmLive.plan.option', { n: i + 1 })}
            className={field}
            value={d}
            onChange={(e) => setDrafts((x) => x.map((v, j) => (j === i ? e.target.value : v)))}
          />
        ))}
        <div className="flex flex-wrap items-center gap-1.5">
          <Btn disabled={drafts.length >= 6} onClick={() => setDrafts((x) => [...x, ''])}>
            {t('gmLive.plan.add')}
          </Btn>
          <label className="flex items-center gap-1 text-caption">
            {t('gmLive.plan.lobby')}
            <input
              type="number"
              min={0}
              max={180}
              className={`${field} w-16`}
              value={lobby}
              onChange={(e) => setLobby(Number(e.target.value))}
            />
          </label>
          <Btn main disabled={options.length === 0} onClick={() => void onPropose(options, lobby)}>
            {t('gmLive.plan.send')}
          </Btn>
        </div>
      </fieldset>
    </Panel>
  )
}
