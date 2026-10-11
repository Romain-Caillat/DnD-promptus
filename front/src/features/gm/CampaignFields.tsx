import { useId, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { field } from '@/features/gm/live/ui'
import type { RulePreset } from '@/lib/campaigns'
import { cn } from '@/lib/utils'
import type { CampaignForm } from './campaignForm'

function Field({
  label,
  hint,
  below,
  children,
}: {
  label: string
  hint?: string
  /** What Claude proposes for the field, or that it just changed. */
  below?: ReactNode
  children: (id: string) => ReactNode
}) {
  const id = useId()
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="text-caption text-mute-soft">
        {label}
      </label>
      {children(id)}
      {hint && <span className="text-caption text-mute">{hint}</span>}
      {below}
    </div>
  )
}

type Change = (patch: Partial<CampaignForm>) => void
/** What to show under a field of « L'idée », by the form's key. */
type Below = (key: keyof CampaignForm) => ReactNode

/**
 * « L'idée » at the widest scale (campaign/write-story-by-map-scale):
 * the title, the universe, the GM's idea, the players' hook, the tone
 * and the themes.
 */
export function IdeaFields({ form, onChange, below }: { form: CampaignForm; onChange: Change; below?: Below }) {
  const { t } = useTranslation()
  return (
    <>
      <Field label={t('gm.newCampaign.world.titleLabel')} below={below?.('title')}>
        {(id) => (
          <input
            id={id}
            className={cn(field, 'type-title text-[15px]')}
            value={form.title}
            placeholder={t('gm.newCampaign.world.titlePlaceholder')}
            maxLength={120}
            onChange={(e) => onChange({ title: e.target.value })}
          />
        )}
      </Field>
      <Field label={t('gm.newCampaign.world.worldLabel')} below={below?.('world')}>
        {(id) => (
          <input
            id={id}
            className={field}
            value={form.world}
            placeholder={t('gm.newCampaign.world.worldPlaceholder')}
            maxLength={200}
            onChange={(e) => onChange({ world: e.target.value })}
          />
        )}
      </Field>
      <Field label={t('gm.newCampaign.pitch.pitchLabel')} below={below?.('pitch')}>
        {(id) => (
          <textarea
            id={id}
            className={cn(field, 'type-narration min-h-24 text-[17px] leading-snug')}
            value={form.pitch}
            placeholder={t('gm.newCampaign.pitch.pitchPlaceholder')}
            onChange={(e) => onChange({ pitch: e.target.value })}
          />
        )}
      </Field>
      <Field label={t('gm.newCampaign.pitch.hookLabel')} below={below?.('playerHook')}>
        {(id) => (
          <textarea
            id={id}
            className={cn(field, 'type-narration min-h-20 text-[17px] leading-snug')}
            value={form.playerHook}
            placeholder={t('gm.newCampaign.pitch.hookPlaceholder')}
            onChange={(e) => onChange({ playerHook: e.target.value })}
          />
        )}
      </Field>
      <Field label={t('gm.idea.toneLabel')} below={below?.('tone')}>
        {(id) => (
          <input
            id={id}
            className={field}
            value={form.tone}
            placeholder={t('gm.idea.tonePlaceholder')}
            onChange={(e) => onChange({ tone: e.target.value })}
          />
        )}
      </Field>
      <Field label={t('gm.idea.themesLabel')} below={below?.('themes')}>
        {(id) => (
          <textarea
            id={id}
            className={cn(field, 'min-h-16')}
            value={form.themes}
            placeholder={t('gm.idea.themesPlaceholder')}
            onChange={(e) => onChange({ themes: e.target.value })}
          />
        )}
      </Field>
    </>
  )
}

/** The table's size and the AI budget: technical settings, folded under « L'idée ». */
export function TableFields({ form, onChange }: { form: CampaignForm; onChange: Change }) {
  const { t } = useTranslation()
  return (
    <div className="grid grid-cols-[90px_1fr] gap-3">
      <Field label={t('gm.newCampaign.pitch.playersLabel')} hint={t('gm.newCampaign.pitch.playersHint')}>
        {(id) => (
          <input
            id={id}
            className={field}
            type="number"
            inputMode="numeric"
            min={1}
            max={12}
            value={form.playerCount}
            onChange={(e) => onChange({ playerCount: e.target.value })}
          />
        )}
      </Field>
      <Field label={t('gm.newCampaign.pitch.budgetLabel')} hint={t('gm.newCampaign.pitch.budgetHint')}>
        {(id) => (
          <input
            id={id}
            className={cn(field, 'max-w-32')}
            inputMode="decimal"
            value={form.budget}
            onChange={(e) => onChange({ budget: e.target.value })}
          />
        )}
      </Field>
    </div>
  )
}

/** The stat names a rule system brings: its six abilities, then HP and AC. */
export function PresetStats({ preset }: { preset: RulePreset }) {
  const { t } = useTranslation()
  const chip = 'rounded-full border border-line-strong px-2 py-0.5 text-[11px] font-semibold text-mute-soft'
  return (
    <div className="flex flex-col gap-1.5">
      <ul className="flex flex-wrap gap-1.5" aria-label={t('gm.newCampaign.rules.abilities')}>
        {preset.abilities.map((a) => (
          <li key={a.abbr} className={chip}>
            {a.abbr} · {a.name}
          </li>
        ))}
      </ul>
      <span className="text-caption text-mute">
        {t('gm.newCampaign.rules.derived', {
          hp: `${preset.hitPoints.abbr} · ${preset.hitPoints.name}`,
          ac: `${preset.armorClass.abbr} · ${preset.armorClass.name}`,
        })}
      </span>
    </div>
  )
}
