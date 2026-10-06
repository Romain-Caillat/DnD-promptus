import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import type { ChangeValue, RuleChange } from '@/lib/rules'

const ARROW = '→'
const DOT = '·'

/** One change between two versions of the rules, as players read it. */
export function RuleChangeLine({ item }: { item: RuleChange }) {
  const { t } = useTranslation()
  const section = t(`rules.changes.sections.${item.section}` as 'rules.changes.sections.check')
  const field = t(`rules.changes.fields.${item.field}` as 'rules.changes.fields.value')
  const head = [section, item.subject].filter(Boolean).join(` ${DOT} `)
  if (item.field === 'added' || item.field === 'removed') {
    return (
      <>
        <b>{head}</b> {field}
      </>
    )
  }
  return (
    <>
      <b>{head}</b> {field} <span>{value(item.from, t)}</span> {ARROW} <b>{value(item.to, t)}</b>
    </>
  )
}

function value(v: ChangeValue | null, t: TFunction): string {
  if (!v) return ''
  switch (v.kind) {
    case 'number':
      return String(v.value)
    case 'text':
      return v.value || t('rules.changes.none')
    case 'flag':
      return t(v.value ? 'rules.changes.yes' : 'rules.changes.no')
    case 'code':
      return t(`rules.codes.${v.value}` as 'rules.codes.not_applied')
  }
}
