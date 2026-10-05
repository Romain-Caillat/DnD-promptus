import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { LineIcon, UI_ICONS, type UiIcon } from './icons'

export interface ArcadeAction {
  label: string
  icon: UiIcon
  onClick?: () => void
  disabled?: boolean
}

/**
 * The combat controls of the phone, like a gamepad (board « Jauges et
 * boutons », piste C, as placed in « Jouer · la soirée de Marc »): one big
 * round button under the thumb plays the selected card, with the item and
 * end-turn buttons smaller on either side. Phone in combat only; elsewhere
 * actions are card buttons. While `busy` (the roll is out), the big
 * button waits.
 */
export function ArcadeCluster({
  main,
  left,
  right,
  busy = false,
  className,
}: {
  main: ArcadeAction
  left?: ArcadeAction
  right?: ArcadeAction
  busy?: boolean
  className?: string
}) {
  const { t } = useTranslation()
  return (
    <div
      role="group"
      aria-label={t('game.arcade')}
      className={cn('grid h-28 grid-cols-[1fr_auto_1fr] items-start', className)}
    >
      <div className="flex justify-center pt-[26px]">{left && <SmallArcade action={left} />}</div>
      <button
        type="button"
        className={cn('gk-arc gk-arc-main', busy && 'gk-busy')}
        style={{ width: 104, height: 104 }}
        onClick={main.onClick}
        disabled={main.disabled || busy}
        aria-busy={busy || undefined}
      >
        <span className="gk-arc-top" style={{ width: 80, height: 80 }}>
          <span className="grid place-items-center">
            <LineIcon paths={UI_ICONS[main.icon]} size={24} strokeWidth={2} />
            <span className="mt-[3px] text-[11px] font-bold tracking-[.08em] uppercase">{main.label}</span>
          </span>
        </span>
      </button>
      <div className="flex justify-center pt-[26px]">{right && <SmallArcade action={right} />}</div>
    </div>
  )
}

function SmallArcade({ action }: { action: ArcadeAction }) {
  return (
    <button
      type="button"
      className="gk-arc-small flex flex-col items-center gap-2.5"
      onClick={action.onClick}
      disabled={action.disabled}
    >
      <span className="gk-arc gk-arc-dark" style={{ width: 58, height: 58 }}>
        <span className="gk-arc-top" style={{ width: 44, height: 44 }}>
          <LineIcon paths={UI_ICONS[action.icon]} size={18} strokeWidth={2} />
        </span>
      </span>
      <span className="w-[90px] text-center text-[11px] text-mute-soft">{action.label}</span>
    </button>
  )
}
