import { createContext, useContext, type ButtonHTMLAttributes, type ReactNode } from 'react'
import { cn } from '@/lib/utils'

/**
 * The live screen is held in the hands (gm/run-on-tablet): every action
 * grows to a finger's size, nothing smaller than a fingertip. The
 * panels read it instead of being copied for the tablet.
 */
export const TouchScreen = createContext(false)

export function useTouchScreen(): boolean {
  return useContext(TouchScreen)
}

/**
 * A small key of the GM screen (pixel menu, ui/adopt-pixel-menu): ivory
 * for the moment's main one, dark for the others; a finger's size on a
 * tablet. Keyboard focus shows the RPG cursor in its left padding.
 */
export function Btn({
  main = false,
  className,
  type = 'button',
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { main?: boolean }) {
  const touch = useTouchScreen()
  return (
    <button
      type={type}
      className={cn(
        'pixel-key pixel-cursor [--px:2px]',
        touch ? 'min-h-12 py-2.5 pr-4 pb-3.5 text-pixel' : 'min-h-8 py-1 pr-2.5 pb-2 text-label',
        !main && 'pixel-key-dark',
        className,
      )}
      {...rest}
    />
  )
}

/**
 * A big key of the tablet: one of the three answers to a proposal, one
 * difficulty for a check. `ivory` when it is the expected one (`main`
 * says the same), `ink` for the black key on an ivory card, `dark`
 * otherwise. Its second line (`sub`) stays in running text.
 */
export function BigKey({
  main = false,
  tone,
  className,
  children,
  sub,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  main?: boolean
  tone?: 'ivory' | 'dark' | 'ink'
  sub?: ReactNode
}) {
  const t = tone ?? (main ? 'ivory' : 'dark')
  return (
    <button
      type="button"
      className={cn(
        'pixel-key pixel-cursor flex min-h-16 flex-col items-center justify-center gap-0.5 py-2 pr-3 pb-3.5 text-pixel',
        t === 'dark' && 'pixel-key-dark',
        t === 'ink' &&
          'text-ivory [--pixel-face:var(--color-ink)] [--pixel-hi:var(--color-ink-soft)] [--pixel-line:var(--color-ink)] [--pixel-low:var(--color-ink-soft)] [--pixel-side:var(--color-ink)]',
        className,
      )}
      {...rest}
    >
      {children}
      {sub && (
        <span
          className={cn(
            'font-sans text-caption font-semibold tracking-normal normal-case',
            t === 'ivory' ? 'text-ink-soft' : 'text-mute-soft',
          )}
        >
          {sub}
        </span>
      )}
    </button>
  )
}

/** One block of the live screen, titled: a pixel panel. */
export function Panel({
  title,
  actions,
  children,
  className,
}: {
  title: string
  actions?: ReactNode
  children: ReactNode
  className?: string
}) {
  return (
    <section className={cn('surface-slab flex flex-col gap-2.5 p-3.5', className)} aria-label={title}>
      <header className="flex items-center justify-between gap-2">
        <h2 className="type-label text-chalk">{title}</h2>
        {actions}
      </header>
      {children}
    </section>
  )
}

/** A field of the GM screen: a pixel box sunk in the table; what is typed stays in running text. */
export const field = 'pixel-field px-2 py-1.5 text-body'
