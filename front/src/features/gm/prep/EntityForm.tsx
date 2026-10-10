import { useMemo, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Btn, Panel } from '@/features/gm/live/ui'
import { valueAt, type Edit } from '@/lib/prep'
import { TextField } from '../fields'

/** A field of an entity form: its dotted path, and how it is written. */
export interface FieldDef {
  path: string
  multiline?: boolean
  number?: boolean
  /** A list of lines (`truths`), edited one per line. */
  lines?: boolean
}

/** The text of `value` as a form shows it. */
function shown(value: unknown, def: FieldDef): string {
  if (def.lines) return Array.isArray(value) ? value.join('\n') : ''
  if (value === undefined || value === null) return ''
  return String(value)
}

/** The value a form's `text` stores. Empty clears the field. */
function stored(text: string, def: FieldDef): unknown {
  if (def.lines) {
    const lines = text
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean)
    return lines.length ? lines : null
  }
  if (def.number) return text.trim() === '' ? null : Number(text)
  return text.trim() === '' ? null : text
}

/** The text fields of `entity`, as typed, with the `set` edits of those changed. */
export function useEntityFields(target: string, entity: Record<string, unknown>, fields: FieldDef[]) {
  const initial = useMemo(
    () => Object.fromEntries(fields.map((f) => [f.path, shown(valueAt(entity, f.path), f)])),
    [entity, fields],
  )
  const [values, setValues] = useState(initial)
  const changed = fields.filter((f) => values[f.path] !== initial[f.path])
  const edits: Edit[] = changed.map((f) => ({ op: 'set', target, field: f.path, value: stored(values[f.path], f) }))
  return { values, setValues, edits }
}

/** The inputs of `fields`, labelled from `prep.review.field.*`. */
export function EntityFields({
  fields,
  values,
  onChange,
}: {
  fields: FieldDef[]
  values: Record<string, string>
  onChange: (path: string, value: string) => void
}) {
  const { t } = useTranslation()
  return fields.map((f) => (
    <TextField
      key={f.path}
      label={t(`prep.review.field.${f.path}`, { defaultValue: f.path })}
      multiline={f.multiline || f.lines}
      value={values[f.path] ?? ''}
      onChange={(v) => onChange(f.path, v)}
    />
  ))
}

/** One entity's fields, saved as `set` edits of the changed ones. */
export function EntityForm({
  title,
  target,
  entity,
  fields,
  busy,
  onSave,
  children,
}: {
  title: string
  target: string
  entity: Record<string, unknown>
  fields: FieldDef[]
  busy: boolean
  onSave: (edits: Edit[]) => Promise<void>
  children?: ReactNode
}) {
  const { t } = useTranslation()
  const { values, setValues, edits } = useEntityFields(target, entity, fields)
  return (
    <Panel title={title}>
      <EntityFields fields={fields} values={values} onChange={(path, v) => setValues((cur) => ({ ...cur, [path]: v }))} />
      <Btn main className="self-end" disabled={busy || edits.length === 0} onClick={() => void onSave(edits)}>
        {t('prep.review.save')}
      </Btn>
      {children}
    </Panel>
  )
}
