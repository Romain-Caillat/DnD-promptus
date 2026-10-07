import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import type { LiveScreen } from '@/lib/evening'
import { cn } from '@/lib/utils'
import { Btn } from './ui'

export type Section = 'scene' | 'table' | 'map' | 'journal'

const SECTIONS: { id: Section; glyph: string }[] = [
  { id: 'scene', glyph: '❖' },
  { id: 'table', glyph: '☰' },
  { id: 'map', glyph: '▦' },
  { id: 'journal', glyph: '✎' },
]
const COPILOT_GLYPH = '✦'

/**
 * The live screen on a tablet (gm/run-on-tablet, planche « Tablette »):
 * a rail of five big targets in place of the three columns — the scene
 * and the requests, the table, the map and the fight, the journal and
 * the end of the evening, and the co-GM, which slides in as a drawer
 * over the section without leaving it. A dot on a target says something
 * waits there. The seats stay on the right while the screen is wide
 * enough: six players, who asks, who waits too long. A seat only opens
 * the table: a stray thumb must not record a moment for a player.
 */
export function TabletScreen({
  screen,
  sections,
  waiting,
  copilot,
}: {
  screen: LiveScreen
  sections: Record<Section, ReactNode>
  /** Where something waits for the GM. */
  waiting: Partial<Record<Section | 'copilot', boolean>>
  copilot: ReactNode
}) {
  const { t } = useTranslation()
  const [current, setCurrent] = useState<Section>('scene')
  const [drawer, setDrawer] = useState(false)
  const target = (on: boolean) =>
    cn(
      'relative flex h-[72px] w-full flex-col items-center justify-center gap-1 rounded-[14px] border text-caption font-semibold',
      on ? 'border-ink bg-ivory text-ink shadow-ivory-flat' : 'border-line bg-surface text-mute-soft',
    )
  const dot = <i aria-hidden className="absolute top-2 right-2 size-2.5 rounded-full bg-stat-init" />
  return (
    <div className="grid flex-1 grid-cols-[84px_minmax(0,1fr)] gap-3 min-[1000px]:grid-cols-[84px_minmax(0,1fr)_280px]">
      <nav aria-label={t('gmLive.tablet.rail')} className="flex flex-col gap-2">
        {SECTIONS.map((s) => (
          <button
            key={s.id}
            type="button"
            aria-current={current === s.id && !drawer ? 'page' : undefined}
            className={target(current === s.id && !drawer)}
            onClick={() => {
              setCurrent(s.id)
              setDrawer(false)
            }}
          >
            <b aria-hidden className="font-title text-[22px]">
              {s.glyph}
            </b>
            {t(`gmLive.tablet.${s.id}`)}
            {waiting[s.id] && dot}
          </button>
        ))}
        <button type="button" aria-expanded={drawer} className={target(drawer)} onClick={() => setDrawer((d) => !d)}>
          <b aria-hidden className="font-title text-[22px]">
            {COPILOT_GLYPH}
          </b>
          {t('gmLive.tablet.copilot')}
          {waiting.copilot && dot}
        </button>
      </nav>
      <div className="flex min-w-0 flex-col gap-3">{sections[current]}</div>
      <Seats
        screen={screen}
        onOpen={() => {
          setCurrent('table')
          setDrawer(false)
        }}
      />
      {drawer && (
        <aside
          aria-label={t('gmLive.tablet.copilot')}
          className="fixed inset-y-0 right-0 z-30 flex w-[440px] max-w-[calc(100vw-96px)] flex-col gap-3 overflow-y-auto border-l border-line-strong bg-table p-4 shadow-[-30px_0_60px_rgba(0,0,0,0.7)] motion-safe:animate-in motion-safe:slide-in-from-right"
        >
          <div className="flex justify-end">
            <Btn onClick={() => setDrawer(false)}>{t('gmLive.tablet.close')}</Btn>
          </div>
          {copilot}
        </aside>
      )}
    </div>
  )
}

/** The seats, big enough for a thumb: who is here, who asks, who waits too long. */
function Seats({ screen, onOpen }: { screen: LiveScreen; onOpen: () => void }) {
  const { t } = useTranslation()
  const seated = screen.lobby.filter((s) => s.role !== 'spectator')
  return (
    <section aria-label={t('gmLive.tablet.seats')} className="surface-slab hidden flex-col gap-1.5 self-start p-2.5 min-[1000px]:flex">
      <header className="flex items-center justify-between px-1">
        <h2 className="type-label text-chalk">{t('gmLive.table.title')}</h2>
        <span className="type-label">{t('gmLive.tablet.here', { here: seated.filter((s) => s.online).length, all: seated.length })}</span>
      </header>
      {seated.map((s) => {
        const spot = screen.spotlight.find((x) => x.playerId === s.playerId)
        return (
          <button
            key={s.playerId}
            type="button"
            className={cn(
              'flex min-h-16 items-center gap-2.5 rounded-xl bg-surface px-3 text-left',
              spot && spot.pendingRequests > 0 && 'border-[1.5px] border-stat-init',
              spot?.alert && 'border-[1.5px] border-stat-atk',
            )}
            onClick={onOpen}
            title={t('gmLive.tablet.see')}
          >
            <span className="flex min-w-0 flex-1 flex-col">
              <b className="truncate text-body">{s.characterName ?? s.nickname}</b>
              <small className="truncate text-caption text-mute-soft">
                {[
                  s.nickname,
                  spot ? t('gmLive.table.idle', { count: spot.idleMinutes }) : null,
                  spot && spot.pendingRequests > 0 ? t('gmLive.table.pending', { count: spot.pendingRequests }) : null,
                ]
                  .filter(Boolean)
                  .join(' · ')}
              </small>
            </span>
            <span
              aria-hidden
              className={cn('size-2.5 shrink-0 rounded-full', s.online ? 'bg-stat-move' : 'border border-line')}
            />
          </button>
        )
      })}
    </section>
  )
}
