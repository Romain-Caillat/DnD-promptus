import { cn } from '@/lib/utils'

/**
 * A faction's affinity, from its `min` to its `max`: one cell per
 * point, filled from zero toward the value — green for favour, red for
 * hostility (the number is shown beside it). `label` names it for
 * screen readers.
 */
export function AffinityGauge({
  value,
  min,
  max,
  label,
}: {
  value: number
  min: number
  max: number
  label: string
}) {
  const cells = []
  for (let v = min; v <= max; v++) {
    if (v === 0) continue
    const lit = value > 0 ? v > 0 && v <= value : v < 0 && v >= value
    cells.push(
      <span
        key={v}
        className={cn(
          'h-2.5 flex-1 rounded-[2px] border border-line',
          lit && (v > 0 ? 'border-transparent bg-stat-move' : 'border-transparent bg-damage'),
          v === 1 && 'ml-1',
        )}
      />,
    )
  }
  return (
    <div
      role="meter"
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      className="flex min-w-24 flex-1 items-center gap-0.5"
    >
      {cells}
      <span className="ml-1.5 w-6 text-right text-caption font-bold tabular-nums">
        {value > 0 ? `+${value}` : value}
      </span>
    </div>
  )
}
