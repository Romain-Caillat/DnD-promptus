import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { useBetween } from './useBetween'

const dateFormat = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'short' })

/**
 * The campaign's chronicle on the Journal tab (planche « Entre deux »,
 * moment 4): one entry per session played — its number and date, the
 * scene it ended in, its « Précédemment… » once published — the last
 * one marked; then the threads still open. Only what the whole table
 * saw. Nothing before the first session ends.
 */
export function Chronicle({ campaignId, refreshKey }: { campaignId: string; refreshKey: number }) {
  const { t } = useTranslation()
  const state = useBetween(campaignId, refreshKey)
  if (state.kind !== 'ready' || state.between.chronicle.length === 0) return null
  const { chronicle, openThreads } = state.between
  return (
    <section className="flex flex-col gap-3" aria-label={t('between.chronicle')}>
      <h2 className="type-label">{t('between.chronicle')}</h2>
      <ol className="ml-1.5 flex flex-col border-l-2 border-line">
        {chronicle.map((entry, i) => (
          <li
            key={entry.number}
            className={cn(
              'relative flex flex-col gap-1 pb-3.5 pl-4.5 text-body',
              'before:absolute before:top-1 before:-left-[7px] before:size-3 before:rounded-[3px] before:shadow-[0_0_0_3px_var(--color-table)]',
              i === chronicle.length - 1 ? 'before:bg-chalk' : 'before:bg-line',
            )}
          >
            <span className="text-[10px] font-bold tracking-[0.14em] text-mute uppercase">
              {entry.endedAt
                ? t('between.entry', { number: entry.number, date: dateFormat.format(new Date(entry.endedAt)) })
                : t('between.entryNoDate', { number: entry.number })}
            </span>
            {entry.title && <b className="type-title text-[16px]">{entry.title}</b>}
            {entry.text ? (
              <p className="text-chalk-soft">{entry.text}</p>
            ) : (
              <p className="text-caption text-mute">{t('between.noText')}</p>
            )}
          </li>
        ))}
      </ol>
      <p className="text-caption text-mute">{t('between.chronicleNote')}</p>
      {openThreads.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <h3 className="type-label">{t('between.open')}</h3>
          <ul className="flex flex-col gap-1.5">
            {openThreads.map((line, i) => (
              <li key={i} className="rounded-button border border-line bg-well px-3 py-2 text-body">
                {line}
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  )
}
