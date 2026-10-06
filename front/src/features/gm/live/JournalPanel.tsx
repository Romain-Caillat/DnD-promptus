import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { JournalKind, LiveScreen } from '@/lib/evening'
import { cn } from '@/lib/utils'
import { Btn, Panel, field } from './ui'

const NOTE_KINDS: JournalKind[] = ['note', 'promise', 'debt', 'item']

/**
 * The table's memory (session/track-table-knowledge): every line, the
 * hidden ones dimmed, and a promise, a debt, a key item or a note to add
 * — shared with the table or kept for the GM.
 */
export function JournalPanel({
  screen,
  onNote,
}: {
  screen: LiveScreen
  onNote: (kind: JournalKind, text: string, shared: boolean) => Promise<void>
}) {
  const { t } = useTranslation()
  const [kind, setKind] = useState<JournalKind>('note')
  const [text, setText] = useState('')
  const [shared, setShared] = useState(true)
  const open = Boolean(screen.session && screen.session.status !== 'ended')
  return (
    <Panel title={t('gmLive.journal.title')}>
      <form
        className="flex flex-col gap-1.5"
        onSubmit={async (e) => {
          e.preventDefault()
          if (!text.trim()) return
          await onNote(kind, text.trim(), shared)
          setText('')
        }}
      >
        <div className="flex gap-1.5">
          <select className={field} value={kind} onChange={(e) => setKind(e.target.value as JournalKind)} aria-label={t('gmLive.journal.kind')}>
            {NOTE_KINDS.map((k) => (
              <option key={k} value={k}>
                {t(`evening.journalKind.${k}`)}
              </option>
            ))}
          </select>
          <input
            className={cn(field, 'min-w-0 flex-1')}
            value={text}
            onChange={(e) => setText(e.target.value)}
            maxLength={1000}
            placeholder={t('gmLive.journal.placeholder')}
            aria-label={t('gmLive.journal.placeholder')}
          />
        </div>
        <div className="flex items-center justify-between gap-2">
          <label className="flex items-center gap-1.5 text-caption">
            <input type="checkbox" checked={shared} onChange={(e) => setShared(e.target.checked)} />
            {t('gmLive.journal.shared')}
          </label>
          <Btn type="submit" disabled={!open || !text.trim()}>
            {t('gmLive.journal.add')}
          </Btn>
        </div>
      </form>
      <ul className="flex max-h-72 flex-col gap-1 overflow-y-auto">
        {[...screen.journal].reverse().map((l) => (
          <li key={l.id} className={cn('text-caption', !l.shared && 'text-mute italic')}>
            <span className="type-label mr-1.5">{t(`evening.journalKind.${l.kind}`)}</span>
            {l.text}
            {!l.shared && ` · ${t('gmLive.journal.hidden')}`}
          </li>
        ))}
      </ul>
    </Panel>
  )
}
