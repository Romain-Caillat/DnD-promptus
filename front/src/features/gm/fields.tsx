import { field } from '@/features/gm/live/ui'
import { cn } from '@/lib/utils'

/** A labelled text input of a GM form, or a text area. */
export function TextField({
  label,
  value,
  onChange,
  wide = false,
  multiline = false,
}: {
  label: string
  value: string
  onChange: (v: string) => void
  wide?: boolean
  multiline?: boolean
}) {
  return (
    <label className={cn('flex flex-col gap-1 text-caption text-mute-soft', wide && 'col-span-full')}>
      {label}
      {multiline ? (
        <textarea className={cn(field, 'min-h-16')} value={value} onChange={(e) => onChange(e.target.value)} />
      ) : (
        <input className={field} value={value} onChange={(e) => onChange(e.target.value)} />
      )}
    </label>
  )
}

/** A labelled number input of a GM form. */
export function NumberField({ label, value, onChange }: { label: string; value: number; onChange: (v: number) => void }) {
  return (
    <label className="flex flex-col gap-1 text-caption text-mute-soft">
      {label}
      <input
        className={cn(field, 'w-24')}
        type="number"
        value={Number.isFinite(value) ? value : 0}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </label>
  )
}
