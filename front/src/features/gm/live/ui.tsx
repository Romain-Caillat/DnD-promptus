import type { ButtonHTMLAttributes, ReactNode } from 'react'
import { cn } from '@/lib/utils'

/** A small action of the GM screen: ivory for the moment's main one. */
export function Btn({
  main = false,
  className,
  type = 'button',
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { main?: boolean }) {
  return (
    <button
      type={type}
      className={cn(
        'rounded-button px-2.5 py-1.5 text-caption font-bold disabled:opacity-40',
        main ? 'bg-ivory text-ink shadow-ivory-flat' : 'border border-line text-chalk hover:bg-surface',
        className,
      )}
      {...rest}
    />
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
