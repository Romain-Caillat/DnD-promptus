import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { FeedbackReport, LiveScreen } from '@/lib/evening'
import { Btn, Panel, field } from './ui'

const QUESTIONS = ['rulesClear', 'hadMoment', 'knowsNext'] as const

/**
 * The end of the evening (session/end-session): the GM's recap, kept for
 * them, and « Précédemment… », published to the players — the co-GM can
 * draft both from the journal (a counted AI call, nothing saved until
 * the GM ends the session).
 */
export function EndPanel({
  screen,
  onEnd,
  onDraft,
}: {
  screen: LiveScreen
  onEnd: (recap: string, previously: string) => Promise<void>
  onDraft: () => Promise<{ recap: string; previously: string } | null>
}) {
  const { t } = useTranslation()
  const [recap, setRecap] = useState('')
  const [previously, setPreviously] = useState('')
  const [busy, setBusy] = useState(false)
  if (!screen.session) return null
  return (
    <Panel title={t('gmLive.end.title')}>
      <label className="flex flex-col gap-1 text-caption">
        {t('gmLive.end.recap')}
        <textarea className={field} rows={3} value={recap} onChange={(e) => setRecap(e.target.value)} />
      </label>
      <label className="flex flex-col gap-1 text-caption">
        {t('gmLive.end.previously')}
        <textarea className={field} rows={3} value={previously} onChange={(e) => setPreviously(e.target.value)} />
      </label>
      <div className="flex flex-wrap gap-1.5">
        <Btn
          disabled={busy || !screen.ai.configured}
          onClick={async () => {
            setBusy(true)
            try {
              const d = await onDraft()
              if (d) {
                setRecap(d.recap)
                setPreviously(d.previously)
              }
            } finally {
              setBusy(false)
            }
          }}
        >
          {t('gmLive.end.draft')}
        </Btn>
        <Btn main disabled={busy} onClick={() => void onEnd(recap.trim(), previously.trim())}>
          {t('gmLive.end.end')}
        </Btn>
      </div>
    </Panel>
  )
}

/**
 * After the evening (session/collect-player-feedback): each player's
 * three answers next to what the server measured — the longest wait,
 * requests refused or contested — the revelations still missing, and
 * what the GM will change.
 */
export function FeedbackPanel({
  load,
  onChanges,
}: {
  load: () => Promise<FeedbackReport>
  onChanges: (text: string) => Promise<void>
}) {
  const { t } = useTranslation()
  const [report, setReport] = useState<FeedbackReport | null>(null)
  const [changes, setChanges] = useState('')
  const [saved, setSaved] = useState(false)
  useEffect(() => {
    let live = true
    void load().then(
      (r) => {
        if (!live) return
        setReport(r)
        setChanges(r.gmChanges)
      },
      () => {},
    )
    return () => {
      live = false
    }
  }, [load])
  if (!report) return null
  return (
    <Panel title={t('gmLive.feedback.title', { number: report.number })}>
      <table className="w-full text-caption">
        <thead>
          <tr className="text-left text-mute-soft">
            <th>{t('gmLive.feedback.player')}</th>
            <th>{t('evening.feedback.rulesClear')}</th>
            <th>{t('evening.feedback.hadMoment')}</th>
            <th>{t('evening.feedback.knowsNext')}</th>
            <th>{t('gmLive.feedback.measured')}</th>
          </tr>
        </thead>
        <tbody>
          {report.players.map((p) => (
            <tr key={p.playerId} className="align-top">
              <td className="font-bold">{p.characterName || p.nickname}</td>
              {QUESTIONS.map((q) => (
                <td key={q}>{p.answers ? t(`evening.feedback.answer.${p.answers[q]}`) : t('gmLive.feedback.noAnswer')}</td>
              ))}
              <td>
                {t('gmLive.feedback.measures', {
                  idle: p.longestIdleMinutes,
                  requests: p.requests,
                  refused: p.refused,
                  contested: p.contested,
                })}
                {p.answers?.comment && <span className="block italic">{p.answers.comment}</span>}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {report.gaps.length > 0 && (
        <ul className="list-disc pl-5 text-caption text-chalk-soft">
          {report.gaps.map((g) => (
            <li key={`${g.node}-${g.revelation}`}>{t('gmLive.feedback.gap', { scene: g.nodeTitle, statement: g.statement })}</li>
          ))}
        </ul>
      )}
      <label className="flex flex-col gap-1 text-caption">
        {t('gmLive.feedback.changes')}
        <textarea
          className={field}
          rows={2}
          value={changes}
          onChange={(e) => {
            setChanges(e.target.value)
            setSaved(false)
          }}
        />
      </label>
      <Btn
        onClick={async () => {
          await onChanges(changes.trim())
          setSaved(true)
        }}
      >
        {saved ? t('gmLive.feedback.saved') : t('gmLive.feedback.save')}
      </Btn>
    </Panel>
  )
}
