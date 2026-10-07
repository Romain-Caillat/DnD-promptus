import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ChronicleSession, RecapTexts, RecapWarning, SessionInfo } from '@/lib/evening'
import { Btn, Panel, field } from './ui'

const textsOf = (s: SessionInfo): RecapTexts => ({
  recap: s.recap,
  previously: s.previously,
  chronicleTitle: s.chronicleTitle,
  chronicle: s.chronicle,
})

/**
 * After the evening (session/write-recaps, planche « Mener », fin): the
 * GM rereads the recaps of the session just ended — their own recap,
 * « Précédemment… » and the chronicle entry, drafted by the server from
 * what happened or rewritten by the co-GM — then publishes. Until then
 * the players see nothing. Names the table should not read (a front,
 * someone not met yet) are flagged, never blocked. Below, the chronicle
 * as it grows, one entry per session.
 */
export function RecapPanel({
  session,
  aiConfigured,
  load,
  onDraft,
  onSave,
}: {
  session: SessionInfo
  aiConfigured: boolean
  load: () => Promise<ChronicleSession[]>
  onDraft: () => Promise<(RecapTexts & { warnings: RecapWarning[] }) | null>
  onSave: (texts: RecapTexts, publish: boolean) => Promise<boolean>
}) {
  const { t } = useTranslation()
  const [texts, setTexts] = useState<RecapTexts>(() => textsOf(session))
  const [warnings, setWarnings] = useState<RecapWarning[]>([])
  const [chronicle, setChronicle] = useState<ChronicleSession[]>([])
  const [busy, setBusy] = useState(false)
  const [saved, setSaved] = useState(false)

  const refresh = useCallback(async () => {
    try {
      const all = await load()
      setChronicle(all)
      setWarnings(all.find((s) => s.id === session.id)?.warnings ?? [])
    } catch {
      // The panel keeps what it shows; the next save retries.
    }
  }, [load, session.id])

  useEffect(() => {
    let live = true
    void load().then(
      (all) => {
        if (!live) return
        setChronicle(all)
        setWarnings(all.find((s) => s.id === session.id)?.warnings ?? [])
      },
      () => {},
    )
    return () => {
      live = false
    }
  }, [load, session.id])

  const edit = (key: keyof RecapTexts) => (value: string) => {
    setTexts((x) => ({ ...x, [key]: value }))
    setSaved(false)
  }

  async function save(publish: boolean) {
    setBusy(true)
    try {
      if (await onSave(texts, publish)) {
        setSaved(true)
        await refresh()
      }
    } finally {
      setBusy(false)
    }
  }

  return (
    <Panel title={t('gmLive.recaps.title', { number: session.number })}>
      <p
        role="status"
        className={
          session.publishedAt
            ? 'rounded-button bg-ivory px-3 py-2 text-caption font-bold text-ink shadow-ivory-flat'
            : 'rounded-button border border-dashed border-line-dashed px-3 py-2 text-caption text-chalk-soft'
        }
      >
        {session.publishedAt ? t('gmLive.recaps.publishedState') : t('gmLive.recaps.draftState')}
      </p>
      <Field label={t('gmLive.recaps.recap')} value={texts.recap} rows={4} onChange={edit('recap')} />
      <Field label={t('gmLive.recaps.previously')} value={texts.previously} rows={5} onChange={edit('previously')} />
      <label className="flex flex-col gap-1 text-caption">
        {t('gmLive.recaps.chronicleTitle')}
        <input
          className={field}
          value={texts.chronicleTitle}
          onChange={(e) => edit('chronicleTitle')(e.target.value)}
        />
      </label>
      <Field label={t('gmLive.recaps.chronicle')} value={texts.chronicle} rows={2} onChange={edit('chronicle')} />
      {warnings.length > 0 && (
        <section className="flex flex-col gap-1" aria-label={t('gmLive.recaps.warnings')}>
          <h3 className="type-label text-stat-init">{t('gmLive.recaps.warnings')}</h3>
          <ul className="flex flex-col gap-1 text-caption text-chalk">
            {warnings.map((w) => (
              <li key={w.name}>{t(`gmLive.recaps.warning.${w.kind}`, { name: w.name })}</li>
            ))}
          </ul>
        </section>
      )}
      <div className="flex flex-wrap gap-1.5">
        <Btn
          disabled={busy || !aiConfigured}
          onClick={async () => {
            setBusy(true)
            try {
              const d = await onDraft()
              if (d) {
                setTexts({
                  recap: d.recap,
                  previously: d.previously,
                  chronicleTitle: d.chronicleTitle,
                  chronicle: d.chronicle,
                })
                setWarnings(d.warnings)
                setSaved(false)
              }
            } finally {
              setBusy(false)
            }
          }}
        >
          {t('gmLive.recaps.draft')}
        </Btn>
        <Btn disabled={busy} onClick={() => void save(false)}>
          {saved ? t('gmLive.recaps.saved') : t('gmLive.recaps.save')}
        </Btn>
        <Btn main disabled={busy || !texts.previously.trim()} onClick={() => void save(true)}>
          {t('gmLive.recaps.publish')}
        </Btn>
      </div>
      {chronicle.length > 0 && (
        <section className="flex flex-col gap-1" aria-label={t('gmLive.recaps.chronicleList')}>
          <h3 className="type-label">{t('gmLive.recaps.chronicleList')}</h3>
          <ol className="flex flex-col gap-1 text-caption">
            {chronicle.map((s) => (
              <li key={s.id} className="flex flex-col">
                <b className="text-chalk">
                  {t('gmLive.recaps.entry', { number: s.number, title: s.chronicleTitle })}
                  {!s.publishedAt && ` · ${t('gmLive.recaps.entryDraft')}`}
                </b>
                {s.chronicle && <span className="text-mute-soft">{s.chronicle}</span>}
              </li>
            ))}
          </ol>
        </section>
      )}
    </Panel>
  )
}

function Field({
  label,
  value,
  rows,
  onChange,
}: {
  label: string
  value: string
  rows: number
  onChange: (v: string) => void
}) {
  return (
    <label className="flex flex-col gap-1 text-caption">
      {label}
      <textarea className={field} rows={rows} value={value} onChange={(e) => onChange(e.target.value)} />
    </label>
  )
}
