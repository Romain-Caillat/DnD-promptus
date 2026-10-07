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

/** A small action of the GM screen: ivory for the moment's main one; a finger's size on a tablet. */
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
        'rounded-button font-bold disabled:opacity-40',
        touch ? 'min-h-12 px-4 py-2.5 text-body' : 'px-2.5 py-1.5 text-caption',
        main ? 'bg-ivory text-ink shadow-ivory-flat' : 'border border-line text-chalk hover:bg-surface',
        className,
      )}
      {...rest}
    />
  )
}

/**
 * A big key of the tablet: one of the three answers to a proposal, one
 * difficulty for a check. Ivory when it is the expected one.
 */
export function BigKey({
  main = false,
  className,
  children,
  sub,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { main?: boolean; sub?: ReactNode }) {
  return (
    <button
      type="button"
      className={cn(
        'flex min-h-16 flex-col items-center justify-center gap-0.5 rounded-xl px-3 py-2 text-body font-bold disabled:opacity-40',
        main ? 'bg-ivory text-ink shadow-ivory-flat' : 'border-[1.5px] border-line bg-surface text-chalk',
        className,
      )}
      {...rest}
    >
      {children}
      {sub && <span className={cn('text-caption font-semibold', main ? 'text-ink-soft' : 'text-mute-soft')}>{sub}</span>}
    </button>
  )
}

/** One block of the live screen, titled. */
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

export const field = 'rounded-button border border-line bg-table px-2 py-1.5 text-body text-chalk'
