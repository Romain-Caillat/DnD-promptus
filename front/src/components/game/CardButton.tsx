import type { ButtonHTMLAttributes } from 'react'
import { cn } from '@/lib/utils'
import { LineIcon, UI_ICONS, type UiIcon } from './icons'
import { StatGem } from './StatGem'
import type { Stat } from './stats'

/** A glyph, not a word. */
const CHEVRON = '›'

/**
 * A button that is a card (board « Composant — bouton-carte »): inner
 * frame, Cinzel title, the stat gem when the action depends on one, a
 * chevron. Ivory for the screen's main action, black for the rest;
 * pressing tips the card forward. A real `<button>`: every native prop
 * (`onClick`, `disabled`, `type`…) passes through.
 */
export function CardButton({
  title,
  subtitle,
  variant = 'ivory',
  size = 'normal',
  gem,
  icon,
  width,
  sheen = false,
  pressed = false,
  className,
  type = 'button',
  ...rest
}: {
  title: string
  /** A second line (« Hache d’armes · 1d12+3 »); not shown when small. */
  subtitle?: string
  /** Ivory for the main action, dark for the others. */
  variant?: 'ivory' | 'dark'
  size?: 'normal' | 'small'
  /** The stat the action rolls, shown as its gem. */
  gem?: { stat: Stat; value: string }
  /** A line icon, when the action rolls no stat. */
  icon?: UiIcon
  /** Width in pixels; full width when omitted. */
  width?: number
  /** A light crosses the card from time to time: the action waits for you. */
  sheen?: boolean
  /** Held down: the card stays tipped (an action being sent). */
  pressed?: boolean
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'title'>) {
  const small = size === 'small'
  return (
    <button
      type={type}
      className={cn(
        'button-card gk-card-button',
        variant === 'dark' && 'button-card-dark',
        sheen && 'gk-sheen',
        pressed && 'gk-pressed',
        className,
      )}
      style={{ width: width ?? '100%', minHeight: small ? 36 : 56, padding: small ? 3 : 6 }}
      {...rest}
    >
      <span
        className="card-frame flex items-center"
        style={{ padding: small ? '0 10px' : '6px 12px', gap: small ? 8 : 12, minHeight: small ? 30 : 44 }}
      >
        {gem && (
          <span className="grid flex-none place-items-center drop-shadow-[0_2px_0_rgb(0_0_0/0.45)]">
            <StatGem stat={gem.stat} value={gem.value} size={small ? 22 : 34} />
          </span>
        )}
        {!gem && icon && (
          <span className="grid flex-none place-items-center">
            <LineIcon paths={UI_ICONS[icon]} size={small ? 16 : 24} strokeWidth={1.9} />
          </span>
        )}
        <span className="min-w-0 flex-1">
          <span className="type-title block leading-[1.05]" style={{ fontSize: small ? 13 : 18 }}>
            {title}
          </span>
          {subtitle && !small && (
            <span className="mt-[3px] block text-[11px] font-semibold text-(--sub-color)">{subtitle}</span>
          )}
        </span>
        {!small && (
          <span aria-hidden className="ml-auto font-title text-[22px] font-extrabold">
            {CHEVRON}
          </span>
        )}
      </span>
    </button>
  )
}
