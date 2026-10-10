import { useId, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import type { RulePreset } from '@/lib/campaigns'
import { cn } from '@/lib/utils'
import type { CampaignForm } from './campaignForm'

const inputClass =
  'w-full pixel-field px-3.5 py-2.5 text-body'

function Field({ label, hint, children }: { label: string; hint?: string; children: (id: string) => ReactNode }) {
  const id = useId()
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="type-label">
        {label}
      </label>
      {children(id)}
      {hint && <span className="text-caption text-mute">{hint}</span>}
    </div>
  )
}

type Change = (patch: Partial<CampaignForm>) => void

/** The title and the universe (board « Préparer », moment 2). */
export function IdentityFields({ form, onChange }: { form: CampaignForm; onChange: Change }) {
  const { t } = useTranslation()
  return (
    <>
      <Field label={t('gm.newCampaign.world.titleLabel')}>
        {(id) => (
          <input
            id={id}
            className={cn(inputClass, 'type-title text-[16px]')}
            value={form.title}
            placeholder={t('gm.newCampaign.world.titlePlaceholder')}
            maxLength={120}
            onChange={(e) => onChange({ title: e.target.value })}
          />
        )}
      </Field>
      <Field label={t('gm.newCampaign.world.worldLabel')}>
        {(id) => (
          <input
            id={id}
            className={inputClass}
            value={form.world}
            placeholder={t('gm.newCampaign.world.worldPlaceholder')}
            maxLength={200}
            onChange={(e) => onChange({ world: e.target.value })}
          />
        )}
      </Field>
    </>
  )
}

/** The pitch, the players' hook, the table size and the AI budget (moment 4). */
export function PitchFields({ form, onChange }: { form: CampaignForm; onChange: Change }) {
  const { t } = useTranslation()
  return (
    <>
      <Field label={t('gm.newCampaign.pitch.pitchLabel')}>
        {(id) => (
          <textarea
            id={id}
            className={cn(inputClass, 'type-narration min-h-28 text-[20px] leading-snug')}
            value={form.pitch}
            placeholder={t('gm.newCampaign.pitch.pitchPlaceholder')}
            onChange={(e) => onChange({ pitch: e.target.value })}
          />
        )}
      </Field>
      <Field label={t('gm.newCampaign.pitch.hookLabel')}>
        {(id) => (
          <textarea
            id={id}
            className={cn(inputClass, 'type-narration min-h-20 text-[18px] leading-snug')}
            value={form.playerHook}
            placeholder={t('gm.newCampaign.pitch.hookPlaceholder')}
            onChange={(e) => onChange({ playerHook: e.target.value })}
          />
        )}
      </Field>
      <div className="grid gap-4 sm:grid-cols-[140px_1fr]">
        <Field label={t('gm.newCampaign.pitch.playersLabel')} hint={t('gm.newCampaign.pitch.playersHint')}>
          {(id) => (
            <input
              id={id}
              className={inputClass}
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
              className={cn(inputClass, 'max-w-40')}
              inputMode="decimal"
              value={form.budget}
              onChange={(e) => onChange({ budget: e.target.value })}
            />
          )}
        </Field>
      </div>
    </>
  )
}

/** The stat names a rule system brings: its six abilities, then HP and AC. */
export function PresetStats({ preset, onIvory = false }: { preset: RulePreset; onIvory?: boolean }) {
  const { t } = useTranslation()
  const chip = cn(
    'rounded-full border px-2 py-0.5 text-[11px] font-semibold',
    onIvory ? 'border-ink text-ink' : 'border-line-strong text-mute-soft',
  )
  return (
    <div className="flex flex-col gap-1.5">
      <ul className="flex flex-wrap gap-1.5" aria-label={t('gm.newCampaign.rules.abilities')}>
        {preset.abilities.map((a) => (
          <li key={a.abbr} className={chip}>
            {a.abbr} · {a.name}
          </li>
        ))}
      </ul>
      <span className={cn('text-caption', onIvory ? 'text-ink-soft' : 'text-mute')}>
        {t('gm.newCampaign.rules.derived', {
          hp: `${preset.hitPoints.abbr} · ${preset.hitPoints.name}`,
          ac: `${preset.armorClass.abbr} · ${preset.armorClass.name}`,
        })}
      </span>
    </div>
  )
}
