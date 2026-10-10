import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useNavigate } from 'react-router'
import { CardButton } from '@/components/game/CardButton'
import { ApiError } from '@/lib/api'
import { createCampaign, listRulePresets, type RulePreset, type RulesRef } from '@/lib/campaigns'
import { cn } from '@/lib/utils'
import { IdentityFields, PitchFields, PresetStats } from './CampaignFields'
import { EMPTY_FORM, settingsOf, type CampaignForm } from './campaignForm'

const STEPS = ['world', 'rules', 'pitch'] as const
type Step = 0 | 1 | 2

type PresetsState = { kind: 'loading' } | { kind: 'signed-out' } | { kind: 'error' } | { kind: 'ready'; presets: RulePreset[] }

/** Errors the page can show, by the server's code. */
const ERROR_CODES = ['TITLE_REQUIRED', 'INVALID_PLAYER_COUNT', 'INVALID_AI_BUDGET', 'UNKNOWN_RULE_SYSTEM'] as const
type ErrorCode = (typeof ERROR_CODES)[number] | 'generic'

function errorCode(err: unknown): ErrorCode {
  if (err instanceof ApiError) {
    const known = ERROR_CODES.find((c) => c === err.code)
    if (known) return known
  }
  return 'generic'
}

/**
 * `/campagnes/nouvelle` — a new, empty campaign in three steps (board
 * « Préparer », moments 2 to 4): its title and universe, the rule
 * system it plays and the stat names it brings, then the pitch, the
 * players' hook, the table size and the AI budget. Nothing is generated:
 * the campaign opens empty, ready to prepare.
 */
export function NewCampaignPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [step, setStep] = useState<Step>(0)
  const [form, setForm] = useState<CampaignForm>(EMPTY_FORM)
  const [rules, setRules] = useState<RulesRef | null>(null)
  const [presets, setPresets] = useState<PresetsState>({ kind: 'loading' })
  const [error, setError] = useState<ErrorCode | null>(null)
  const [creating, setCreating] = useState(false)

  useEffect(() => {
    let live = true
    listRulePresets().then(
      (list) => {
        if (!live) return
        setPresets({ kind: 'ready', presets: list })
        setRules((current) => current ?? (list[0] ? { id: list[0].id, version: list[0].version } : null))
      },
      (err: unknown) => {
        if (!live) return
        setPresets(err instanceof ApiError && err.status === 401 ? { kind: 'signed-out' } : { kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [])

  if (presets.kind === 'signed-out') return <Navigate to="/connexion" replace />

  const change = (patch: Partial<CampaignForm>) => {
    setForm((f) => ({ ...f, ...patch }))
    setError(null)
  }

  async function next() {
    setError(null)
    if (step === 0) {
      if (!form.title.trim()) setError('TITLE_REQUIRED')
      else setStep(1)
      return
    }
    if (step === 1) {
      if (rules) setStep(2)
      return
    }
    const settings = settingsOf(form)
    if (typeof settings === 'string') {
      setError(settings)
      return
    }
    if (!rules) return
    setCreating(true)
    try {
      const created = await createCampaign({ ...settings, rules })
      navigate(`/campagnes/${encodeURIComponent(created.id)}`)
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) {
        navigate('/connexion', { replace: true })
        return
      }
      setError(errorCode(err))
      setCreating(false)
    }
  }

  const last = step === 2
  const heading = t(`gm.newCampaign.${STEPS[step]}.title`)
  const kicker = t(`gm.newCampaign.${STEPS[step]}.kicker`)

  return (
    <main className="surface-table flex min-h-dvh flex-col text-chalk">
      <header className="flex flex-wrap items-center gap-4 border-b border-line px-5 py-3 text-caption text-mute-soft">
        <Link className="underline underline-offset-4" to="/">
          {t('gm.newCampaign.back')}
        </Link>
        <span className="type-title text-[16px] text-chalk">{t('gm.newCampaign.title')}</span>
      </header>
      <nav aria-label={t('gm.newCampaign.title')} className="border-b border-line bg-table px-5 py-3">
        <ol className="flex items-center gap-3">
          {STEPS.map((s, i) => (
            <li
              key={s}
              aria-current={i === step ? 'step' : undefined}
              className={cn(
                'flex items-center gap-2 text-[11px] font-semibold tracking-[0.08em] whitespace-nowrap uppercase',
                i === step ? 'text-chalk' : i < step ? 'text-mute-soft' : 'text-ink-soft',
              )}
            >
              {i > 0 && <span aria-hidden className="h-px w-8 border-t border-dashed border-line-strong sm:w-16" />}
              <i
                aria-hidden
                className={cn('size-[9px] rotate-45 border-[1.5px] border-current', i <= step && 'bg-current')}
              />
              {t(`gm.newCampaign.steps.${s}`)}
            </li>
          ))}
        </ol>
      </nav>

      <div className="mx-auto flex w-full max-w-5xl flex-1 flex-col gap-5 p-5">
        <div className="flex flex-wrap items-baseline gap-4">
          <h1 className="type-title text-heading">{heading}</h1>
          <span className="type-label">{kicker}</span>
        </div>

        {step === 0 && (
          <div className="flex max-w-2xl flex-col gap-4">
            <IdentityFields form={form} onChange={change} />
            <p className="text-caption text-mute">{t('gm.newCampaign.world.note')}</p>
          </div>
        )}

        {step === 1 && (
          <div className="flex flex-col gap-4">
            <span className="type-label">{t('gm.newCampaign.rules.pick')}</span>
            {presets.kind === 'loading' && <p role="status">{t('gm.home.loading')}</p>}
            {presets.kind === 'error' && <p role="alert">{t('gm.newCampaign.rules.loadError')}</p>}
            {presets.kind === 'ready' && (
              <div className="grid gap-4 md:grid-cols-2">
                {presets.presets.map((p) => {
                  const on = rules?.id === p.id && rules.version === p.version
                  return (
                    <button
                      key={`${p.id}@${p.version}`}
                      type="button"
                      aria-pressed={on}
                      onClick={() => setRules({ id: p.id, version: p.version })}
                      className={cn(
                        'pixel-choice flex flex-col gap-3 py-4 pr-4 pb-5 text-left [--cursor-room:22px] [--px:3px]',
                      )}
                    >
                      <span className="type-title text-[24px]">{p.name}</span>
                      <span className={cn('text-body', on ? 'text-ink-soft' : 'text-mute-soft')}>{p.description}</span>
                      <PresetStats preset={p} onIvory={on} />
                    </button>
                  )
                })}
              </div>
            )}
            <p className="text-caption text-mute">{t('gm.newCampaign.rules.note')}</p>
          </div>
        )}

        {step === 2 && (
          <div className="flex max-w-3xl flex-col gap-4">
            <PitchFields form={form} onChange={change} />
          </div>
        )}

        {error && (
          <p role="alert" className="text-body text-stat-atk">
            {t(`gm.newCampaign.errors.${error}`)}
          </p>
        )}
      </div>

      <footer className="flex flex-wrap items-center justify-end gap-4 px-5 pb-5">
        {step > 0 && (
          <CardButton
            variant="dark"
            size="small"
            width={140}
            title={t('gm.newCampaign.previous')}
            onClick={() => {
              setError(null)
              setStep((s) => (s - 1) as Step)
            }}
          />
        )}
        <CardButton
          className="max-w-[460px]"
          title={last ? (creating ? t('gm.newCampaign.creating') : t('gm.newCampaign.create')) : t('gm.newCampaign.next')}
          subtitle={last ? t('gm.newCampaign.createSub') : t('gm.newCampaign.nextSub', { step: step + 1 })}
          disabled={creating || (step === 1 && !rules)}
          onClick={() => void next()}
        />
      </footer>
    </main>
  )
}
