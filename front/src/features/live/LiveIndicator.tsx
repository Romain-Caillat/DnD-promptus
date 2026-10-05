import { useTranslation } from 'react-i18next'
import type { LiveStatus } from '@/lib/live'

/**
 * The state of the live channel: quiet while live, outlined while it
 * (re)connects — what is on screen may then be late, and the page
 * catches up by itself once back.
 */
export function LiveIndicator({ status }: { status: LiveStatus }) {
  const { t } = useTranslation()
  const live = status === 'live'
  return (
    <p
      role="status"
      aria-live="polite"
      className={
        live
          ? 'inline-flex items-center gap-2 text-xs text-muted-foreground'
          : 'inline-flex items-center gap-2 border border-current px-2 py-1 text-xs font-semibold'
      }
    >
      <span
        aria-hidden
        className={live ? 'size-2 bg-current' : 'size-2 border border-current motion-safe:animate-pulse'}
      />
      {t(`live.status.${status}`)}
    </p>
  )
}
