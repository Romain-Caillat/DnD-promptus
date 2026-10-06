import { useEffect, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useNavigate, useParams } from 'react-router'
import { StatusBanner } from '@/components/game/StatusBanner'
import { Btn, Panel } from '@/features/gm/live/ui'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { ApiError } from '@/lib/api'
import {
  applyGeneration,
  dollars,
  fetchGenerations,
  startGeneration,
  type GenerationDesk,
  type GenerationInput,
  type GenerationJob,
  type GenerationLength,
  type GenerationStep,
} from '@/lib/generation'
import { cn } from '@/lib/utils'
import { TextField } from '../fields'

const LENGTHS: GenerationLength[] = ['one_shot', 'short', 'long']
const EMPTY: GenerationInput = { pitch: '', tone: '', themes: '', constraints: '', length: 'one_shot' }
const PITCH_MIN = 10
const STEP_MARK: Record<GenerationStep['status'], string> = { pending: '○', running: '✱', done: '✓', failed: '✗' }
const COUNTED = ['nodes', 'revelations', 'clues', 'npcs', 'adversaries', 'locations'] as const

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; desk: GenerationDesk }

/**
 * `/campagnes/:campaignId/generer` — `ai/generate-campaign`: the GM
 * writes an idea, sees what a generation costs at most, starts it and
 * follows its steps live; the draft says what it holds and what the
 * validator finds, and is applied only on the GM's word — then to
 * review on `/preparer`.
 */
export function GenerationPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [version, setVersion] = useState(0)
  const [input, setInput] = useState<GenerationInput>(EMPTY)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    fetchGenerations(campaignId).then(
      (desk) => {
        if (live) setState({ kind: 'ready', desk })
      },
      (err: unknown) => {
        if (!live) return
        if (err instanceof ApiError && err.status === 401) setState({ kind: 'signed-out' })
        else if (err instanceof ApiError && err.status === 404) setState({ kind: 'not-found' })
        else setState({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [campaignId, version])

  useLiveChanges(campaignId, (topics) => {
    if (topics.includes('desk')) setVersion((v) => v + 1)
  })

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />
  const back = (
    <Link className="text-caption text-mute-soft underline underline-offset-4" to={`/campagnes/${campaignId}`}>
      {t('prep.back')}
    </Link>
  )
  if (state.kind !== 'ready') {
    return (
      <main className="surface-table flex min-h-dvh flex-col gap-4 p-6 text-chalk">
        {back}
        {state.kind === 'loading' && <p role="status">{t('prep.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('prep.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('prep.error')}</p>}
      </main>
    )
  }

  const { desk } = state
  const running = desk.jobs.some((j) => j.status === 'running')
  const left = Math.max(0, desk.spending.budgetMicros - desk.spending.spentMicros)
  const tooShort = input.pitch.trim().length < PITCH_MIN

  async function act(run: () => Promise<void>) {
    setBusy(true)
    setError(null)
    try {
      await run()
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'action')
    } finally {
      setBusy(false)
    }
  }

  function submit(e: FormEvent) {
    e.preventDefault()
    void act(async () => {
      await startGeneration(campaignId, input)
      setVersion((v) => v + 1)
    })
  }

  function apply(job: GenerationJob) {
    void act(async () => {
      await applyGeneration(campaignId, job.id)
      navigate(`/campagnes/${campaignId}/preparer`)
    })
  }

  const set = (key: keyof GenerationInput) => (value: string) => setInput((i) => ({ ...i, [key]: value }))

  return (
    <main className="surface-table flex min-h-dvh flex-col gap-4 p-5 text-chalk">
      {back}
      <h1 className="type-title text-[22px]">{t('prep.generate.title')}</h1>
      {desk.validated && <StatusBanner tone="warn">{t('prep.generate.validated')}</StatusBanner>}
      {error && (
        <p role="alert" className="text-body text-stat-atk">
          {t(`prep.generate.errors.${error}`, { defaultValue: t('prep.generate.errors.action') })}
        </p>
      )}

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <Panel title={t('prep.generate.formTitle')}>
          <form className="flex flex-col gap-3" onSubmit={submit}>
            <TextField label={t('prep.generate.pitch')} value={input.pitch} onChange={set('pitch')} multiline />
            <TextField label={t('prep.generate.tone')} value={input.tone} onChange={set('tone')} />
            <TextField label={t('prep.generate.themes')} value={input.themes} onChange={set('themes')} />
            <TextField
              label={t('prep.generate.constraints')}
              value={input.constraints}
              onChange={set('constraints')}
              multiline
            />
            <fieldset className="flex flex-col gap-1">
              <legend className="text-caption text-mute-soft">{t('prep.generate.length')}</legend>
              <div className="flex flex-wrap gap-2">
                {LENGTHS.map((l) => (
                  <label
                    key={l}
                    className={cn(
                      'cursor-pointer rounded-button border px-2.5 py-1.5 text-caption font-semibold',
                      input.length === l ? 'border-ivory bg-ivory text-ink' : 'border-line text-chalk',
                    )}
                  >
                    <input
                      type="radio"
                      name="length"
                      className="sr-only"
                      checked={input.length === l}
                      onChange={() => setInput((i) => ({ ...i, length: l }))}
                    />
                    {t(`prep.generate.lengths.${l}`)}
                  </label>
                ))}
              </div>
            </fieldset>
            <p className="text-caption text-mute-soft">
              {t('prep.generate.cost', { estimate: dollars(desk.estimateMicros), left: dollars(left) })}
            </p>
            <Btn main type="submit" disabled={busy || running || desk.validated || tooShort}>
              {running ? t('prep.generate.running') : t('prep.generate.start')}
            </Btn>
          </form>
        </Panel>

        <section className="flex min-w-0 flex-col gap-4" aria-label={t('prep.generate.jobs')}>
          {desk.jobs.length === 0 && <p className="text-body text-mute-soft">{t('prep.generate.none')}</p>}
          {desk.jobs.map((job, i) => (
            <JobCard
              key={job.id}
              job={job}
              latest={i === 0}
              busy={busy || desk.validated}
              campaignId={campaignId}
              onApply={() => apply(job)}
            />
          ))}
        </section>
      </div>
    </main>
  )
}

const STEP_LABEL = {
  cast: 'prep.generate.steps.cast',
  scenes: 'prep.generate.steps.scenes',
  check: 'prep.generate.steps.check',
} as const

/** A step, and what it produced once done. */
function StepText({ step }: { step: GenerationStep }) {
  const { t } = useTranslation()
  const label = t(STEP_LABEL[step.id])
  if (step.status !== 'done') return label
  const c = step.counts
  let counts: string
  if (step.id === 'cast') {
    counts = t('prep.generate.counts.cast', { npcs: c.npcs ?? 0, adversaries: c.adversaries ?? 0, locations: c.locations ?? 0 })
  } else if (step.id === 'scenes') {
    counts = t('prep.generate.counts.scenes', { nodes: c.nodes ?? 0, clues: c.clues ?? 0 })
  } else {
    counts = t('prep.generate.counts.check', { errors: c.errors ?? 0, warnings: c.warnings ?? 0, repairs: c.repairs ?? 0 })
  }
  return t('prep.generate.stepDone', { label, counts })
}

function JobCard({
  job,
  latest,
  busy,
  campaignId,
  onApply,
}: {
  job: GenerationJob
  latest: boolean
  busy: boolean
  campaignId: string
  onApply: () => void
}) {
  const { t } = useTranslation()
  const errors = job.issues.filter((i) => i.severity === 'error').length
  const draft = job.draft
  return (
    <Panel
      title={t(`prep.generate.status.${job.status}`)}
      className={cn(!latest && 'opacity-80')}
      actions={<span className="text-caption text-mute-soft">{t('prep.generate.spent', { cost: dollars(job.costMicros) })}</span>}
    >
      <p className="text-caption text-mute-soft">{job.input.pitch}</p>
      <ol className="flex flex-col gap-1 text-body">
        {job.steps.map((s) => (
          <li key={s.id} className={cn(s.status === 'failed' && 'text-stat-atk', s.status === 'pending' && 'text-mute-soft')}>
            <span aria-hidden className="mr-2">
              {STEP_MARK[s.status]}
            </span>
            <StepText step={s} />
          </li>
        ))}
      </ol>
      {job.status === 'failed' && (
        <StatusBanner tone="hurt">
          {t(`prep.generate.errors.${job.error ?? 'action'}`, { defaultValue: t('prep.generate.errors.action') })}
          {job.detail && <span className="block text-caption">{job.detail}</span>}
        </StatusBanner>
      )}
      {draft && job.status === 'succeeded' && (
        <>
          {typeof draft.bible.pitch === 'string' && <p className="text-body">{draft.bible.pitch}</p>}
          <ul className="flex flex-wrap gap-x-4 gap-y-1 text-caption text-mute-soft">
            {COUNTED.map((k) => (
              <li key={k}>{t(`prep.generate.drafted.${k}`, { count: draft[k]?.length ?? 0 })}</li>
            ))}
          </ul>
          <ul className="flex flex-col gap-0.5 text-caption">
            {(draft.nodes ?? []).map((n) => (
              <li key={n.id}>
                {n.title}
                {n.optional && <span className="text-mute-soft">{t('prep.generate.optional')}</span>}
              </li>
            ))}
          </ul>
          {job.issues.length === 0 ? (
            <StatusBanner tone="listen">{t('prep.generate.green')}</StatusBanner>
          ) : (
            <StatusBanner tone="warn">
              {t('prep.generate.issues', { errors, warnings: job.issues.length - errors })}
            </StatusBanner>
          )}
          {job.removed.length + job.dropped > 0 && (
            <details className="text-caption text-mute-soft">
              <summary>{t('prep.generate.removed', { count: job.removed.length + job.dropped })}</summary>
              <ul>
                {job.removed.map((r) => (
                  <li key={`${r.code}${r.path}`}>{r.path}</li>
                ))}
              </ul>
            </details>
          )}
          <Btn main disabled={busy} onClick={onApply}>
            {t('prep.generate.apply')}
          </Btn>
          <p className="text-caption text-mute-soft">{t('prep.generate.applyHint')}</p>
        </>
      )}
      {job.status === 'applied' && (
        <Link className="text-caption underline underline-offset-4" to={`/campagnes/${campaignId}/preparer`}>
          {t('prep.generate.review')}
        </Link>
      )}
    </Panel>
  )
}
