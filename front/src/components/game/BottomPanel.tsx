import type { ReactNode } from 'react'
import { Drawer } from '@base-ui/react/drawer'
import { cn } from '@/lib/utils'

/**
 * A panel that slides up from the bottom over the screen (the bag, the
 * character, the map of « Jouer · la soirée de Marc »), without leaving
 * it. Controlled: the screen owns `open`. Swiping it down, tapping the
 * table behind it or Escape closes it; focus stays inside while open.
 */
export function BottomPanel({
  open,
  onOpenChange,
  title,
  children,
  className,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Its name, shown at the top and read by screen readers. */
  title: string
  children: ReactNode
  className?: string
}) {
  return (
    <Drawer.Root open={open} onOpenChange={(next) => onOpenChange(next)}>
      <Drawer.Portal>
        <Drawer.Backdrop className="gk-panel-backdrop" />
        <Drawer.Viewport className="fixed inset-0 flex items-end justify-center">
          <Drawer.Popup className={cn('gk-panel', className)}>
            <span aria-hidden className="gk-panel-grip" />
            <Drawer.Title className="type-title text-[18px]">{title}</Drawer.Title>
            <Drawer.Content className="flex flex-col gap-4">{children}</Drawer.Content>
          </Drawer.Popup>
        </Drawer.Viewport>
      </Drawer.Portal>
    </Drawer.Root>
  )
}
