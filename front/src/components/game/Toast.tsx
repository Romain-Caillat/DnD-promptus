import { useRef, useState, type PointerEvent, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

/** `good` news is ivory, `bad` news black, plain `info` outlined. */
export type ToastTone = 'good' | 'bad' | 'info'

/** How far a swipe to the right goes before the toast is put away. */
const SWIPE_AWAY = 80

/**
 * A phone notification (board « Notifications »): a card that drops from
 * the top and stacks under the previous ones. Ivory for good news, black
 * for what hinders you, outlined for information. A swipe to the right
 * puts it away (everything stays in the journal); keyboard and screen
 * reader users get a « Ranger » button instead.
 */
export function Toast({
  tone,
  kicker,
  media,
  children,
  onDismiss,
  className,
}: {
  tone: ToastTone
  /** The small capitals line (« Butin · rare »). */
  kicker: string
  /** An item slot, a condition badge, a gem… on the left. */
  media?: ReactNode
  children: ReactNode
  onDismiss?: () => void
  className?: string
}) {
  const { t } = useTranslation()
  const start = useRef<number | null>(null)
  const [dx, setDx] = useState(0)

  const swipe = onDismiss
    ? {
        onPointerDown: (e: PointerEvent<HTMLDivElement>) => {
          start.current = e.clientX
          e.currentTarget.setPointerCapture?.(e.pointerId)
        },
        onPointerMove: (e: PointerEvent<HTMLDivElement>) => {
          if (start.current !== null) setDx(Math.max(0, e.clientX - start.current))
        },
        onPointerUp: () => {
          start.current = null
          if (dx > SWIPE_AWAY) onDismiss()
          else setDx(0)
        },
        onPointerCancel: () => {
          start.current = null
          setDx(0)
        },
      }
    : {}

  return (
    <div
      className={cn('gk-toast', className)}
      data-tone={tone}
      style={dx ? { transform: `translateX(${dx}px)`, opacity: Math.max(0.3, 1 - dx / 240) } : undefined}
      {...swipe}
    >
      {media && <span className="flex-none">{media}</span>}
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="gk-toast-kicker">{kicker}</span>
        <span className="text-[13px] leading-[1.3]">{children}</span>
      </div>
      {onDismiss && (
        <button type="button" className="sr-only focus:not-sr-only focus:text-caption" onClick={onDismiss}>
          {t('game.toast.dismiss')}
        </button>
      )}
    </div>
  )
}

/**
 * Toasts stacked from the top, newest last, announced politely to screen
 * readers. Place it (absolute or fixed) where the screen wants them.
 */
export function ToastStack({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div aria-live="polite" className={cn('flex flex-col gap-4', className)}>
      {children}
    </div>
  )
}
