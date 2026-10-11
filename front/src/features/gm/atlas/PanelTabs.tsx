import { useId, useRef, type KeyboardEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

/** The Atlas's side panel, one tab at a time (campaign/tidy-atlas-layout), kept in `?panneau=`. */
const PANEL_TABS =['idee', 'carte', 'claude'] as const
export type PanelTab = (typeof PANEL_TABS)[number]

/** Read `?panneau=`: an unknown value is no choice at all. */
export const panelTabOf = (value: string | null): PanelTab | null => PANEL_TABS.find((x) => x === value) ?? null

/**
 * The tab shown: the GM's choice when it is offered here, else « L'idée »
 * at the widest scale and « La carte » elsewhere. A choice of « L'idée »
 * survives a walk into a place, and comes back on the way out.
 */
export function shownPanelTab(asked: PanelTab | null, widest: boolean): PanelTab {
  if (asked === 'idee' && !widest) return 'carte'
  return asked ?? (widest ? 'idee' : 'carte')
}

/**
 * The tabs on top of the side panel, as an ARIA tab list (arrows, Home
 * and End move between them), and the open one's panel. « Claude » wears
 * the count of its batches waiting: a new one lights it, never opens it.
 */
export function PanelTabs({
  tabs,
  current,
  waiting,
  aside,
  onTab,
  children,
}: {
  tabs: readonly PanelTab[]
  current: PanelTab
  /** Claude's batches waiting on the GM. */
  waiting: number
  /** At the end of the row (the tablet's « Replier »). */
  aside?: ReactNode
  onTab: (tab: PanelTab) => void
  children: ReactNode
}) {
  const { t } = useTranslation()
  const base = useId()
  const refs = useRef(new Map<PanelTab, HTMLButtonElement>())
  const tabId = (x: PanelTab) => `${base}-tab-${x}`
  const panelId = `${base}-panel`

  function onKeyDown(e: KeyboardEvent<HTMLButtonElement>) {
    const i = tabs.indexOf(current)
    const to =
      e.key === 'ArrowRight'
        ? tabs[(i + 1) % tabs.length]
        : e.key === 'ArrowLeft'
          ? tabs[(i - 1 + tabs.length) % tabs.length]
          : e.key === 'Home'
            ? tabs[0]
            : e.key === 'End'
              ? tabs[tabs.length - 1]
              : undefined
    if (!to) return
    e.preventDefault()
    onTab(to)
    refs.current.get(to)?.focus()
  }

  return (
    <div className="flex min-w-0 flex-col gap-2">
      <div className="flex items-center gap-1.5">
        <div role="tablist" aria-label={t('atlas.panel.tabs')} className="flex flex-1 flex-wrap gap-1">
          {tabs.map((x) => (
            <button
              key={x}
              ref={(el) => {
                if (el) refs.current.set(x, el)
                else refs.current.delete(x)
              }}
              id={tabId(x)}
              type="button"
              role="tab"
              aria-selected={x === current}
              aria-controls={panelId}
              tabIndex={x === current ? 0 : -1}
              aria-label={x === 'claude' && waiting > 0 ? t('atlas.panel.claudeWaiting', { count: waiting }) : undefined}
              className="pixel-tab flex min-h-9 items-center gap-1.5 pb-1"
              onClick={() => onTab(x)}
              onKeyDown={onKeyDown}
            >
              {t(`atlas.panel.${x}`)}
              {x === 'claude' && waiting > 0 && (
                <span
                  aria-hidden="true"
                  className={cn(
                    'grid min-w-5 place-items-center px-1 font-sans text-caption leading-5 font-bold',
                    x === current ? 'bg-ink text-ivory' : 'bg-ivory text-ink',
                  )}
                >
                  {waiting}
                </span>
              )}
            </button>
          ))}
        </div>
        {aside}
      </div>
      <div role="tabpanel" id={panelId} aria-labelledby={tabId(current)} className="flex min-w-0 flex-col gap-3">
        {children}
      </div>
    </div>
  )
}
