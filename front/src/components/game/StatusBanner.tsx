import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

/**
 * - `listen`: the GM tells, nothing to do;
 * - `wait`: someone else is acting;
 * - `you`: the player has something to do (it breathes);
 * - `turn`: it is the player's combat turn (yellow, it breathes faster);
 * - `star`: a find, a reward;
 * - `hurt`: the player was hit;
 * - `warn`: something needs a look (GM screens).
 */
export type BannerTone = 'listen' | 'wait' | 'you' | 'turn' | 'star' | 'hurt' | 'warn'

/**
 * The status banner every screen opens with (« Jouer · la soirée de
 * Marc »): one line saying what is happening and whether it is up to
 * you. Announced politely to screen readers when it changes.
 */
export function StatusBanner({
  tone,
  icon,
  children,
  className,
}: {
  tone: BannerTone
  /** A small glyph or pixel icon before the text. */
  icon?: ReactNode
  children: ReactNode
  className?: string
}) {
  return (
    <div role="status" className={cn('gk-banner', className)} data-tone={tone}>
      {icon && (
        <span aria-hidden className="grid flex-none place-items-center">
          {icon}
        </span>
      )}
      <span>{children}</span>
    </div>
  )
}
