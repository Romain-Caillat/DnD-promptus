import type { ButtonHTMLAttributes, CSSProperties } from 'react'
import { cn } from '@/lib/utils'
import { LineIcon, UI_ICONS, type UiIcon } from './icons'
import { StatGem } from './StatGem'
import type { Stat } from './stats'

/** A glyph, not a word. */
const CHEVRON = '›'

/**
 * A key of the pixel menu (board « Pistes UI », track A,
 * ui/adopt-pixel-menu): stepped corners, flat relief, a Silkscreen label,
 * the stat gem when the action depends on one, a chevron. Ivory for the
 * screen's main action, dark for the rest; pressing sinks it into the
 * table. Its left padding holds the RPG cursor, shown on keyboard focus
 * and when `aria-pressed`/`aria-current` mark it as the current choice.
 * A real `<button>`: every native prop (`onClick`, `disabled`, `type`…)
 * passes through.
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
  /** Held down: the key stays sunk (an action being sent). */
  pressed?: boolean
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'title'>) {
  const small = size === 'small'
  return (
    <button
      type={type}
      className={cn(
        'button-card gk-card-button pixel-cursor',
        variant === 'dark' && 'button-card-dark',
        sheen && 'gk-sheen',
        pressed && 'gk-pressed',
        className,
      )}
      style={
        {
          '--px': small ? '2px' : '3px',
          width: width ?? '100%',
          minHeight: small ? 36 : 56,
          padding: small ? '2px 4px 6px 18px' : '6px 6px 9px 22px',
        } as CSSProperties
      }
      {...rest}
    >
      <span
        className="card-frame flex items-center"
        style={{ padding: small ? '0 6px 0 0' : '4px 8px 4px 0', gap: small ? 8 : 12, minHeight: small ? 28 : 41 }}
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
          <span className="type-key block leading-[1.2]" style={{ fontSize: small ? 12 : 16 }}>
            {title}
          </span>
          {subtitle && !small && (
            <span className="mt-[3px] block text-[11px] font-semibold text-(--sub-color)">{subtitle}</span>
          )}
        </span>
        {!small && (
          <span aria-hidden className="ml-auto font-pixel text-[24px] leading-none">
            {CHEVRON}
          </span>
        )}
      </span>
    </button>
  )
}
