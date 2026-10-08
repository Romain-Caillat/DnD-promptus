import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Navigate, useNavigate, useParams } from 'react-router'
import { CardButton } from '@/components/game/CardButton'
import { StatusBanner } from '@/components/game/StatusBanner'
import type { CharacterLook } from '@/features/sprites/look'
import { ApiError } from '@/lib/api'
import {
  fetchCreation,
  fetchPack,
  overBudget,
  saveCharacter,
  submitCharacter,
  type Creation,
  type PackCatalogue,
} from '@/lib/creator'
import { fetchPlayerHome, playPath, type CharacterSheet, type CharacterView, type PlayerHome } from '@/lib/play'
import { AppearanceStep, type AppearanceTab } from './AppearanceStep'
import { CharacterSummary } from './ReviewStep'
import { AbilitiesStep, ClassStep, OptionGrid, StoryStep } from './RuleSteps'
import './creator.css'

type Step = 'people' | AppearanceTab | 'class' | 'abilities' | 'story' | 'review'

type LoadState =
  | { kind: 'loading' }
  | { kind: 'not-joined' }
  | { kind: 'error' }
  | { kind: 'ready'; home: PlayerHome; character: CharacterView; creation: Creation; pack: PackCatalogue }

/** The steps a campaign's rules ask for, in order. */
function stepsFor(creation: Creation): Step[] {
  const r = creation.rules
  return [
    ...(r?.peoples.length ? (['people'] as const) : []),
    'body',
    'outfit',
    'colours',
    ...(r?.classes.length ? (['class'] as const) : []),
    ...(r?.abilities.length && r.classes.length ? (['abilities'] as const) : []),
    'story',
    'review',
  ]
}

/** The four parts the progress dots count (board « Créer »). */
function partOf(step: Step): 'look' | 'class' | 'abilities' | 'story' {
  if (step === 'class' || step === 'abilities') return step
  if (step === 'story' || step === 'review') return 'story'
  return 'look'
}

type ErrorKey = 'locked' | 'name' | 'tooLong' | 'incomplete' | 'generic'

function errorKey(err: unknown): ErrorKey {
  if (err instanceof ApiError) {
    if (err.code === 'CHARACTER_LOCKED') return 'locked'
    if (err.code === 'INVALID_NAME') return 'name'
    if (err.code === 'TEXT_TOO_LONG') return 'tooLong'
    if (err.code === 'CHARACTER_INCOMPLETE') return 'incomplete'
  }
  return 'generic'
}

function clock(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000))
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`
}

/**
 * `/partie/:campaignId/personnage` — the player builds their character
 * on their phone (board « Créer · le personnage de Marc »): the look
 * first, layer by layer, then what the campaign's rules ask — people,
 * class, abilities — a short story, and it goes to the GM. Every step
 * saves the draft, so a closed tab resumes where it was.
 */
export function CharacterCreatorPage() {
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<LoadState>({ kind: 'loading' })

  useEffect(() => {
    let live = true
    async function load(): Promise<LoadState> {
      const home = await fetchPlayerHome(campaignId)
      if (!home) return { kind: 'not-joined' }
      if (!home.character) return { kind: 'not-joined' }
      const creation = await fetchCreation(campaignId)
      const pack = await fetchPack(creation.pack)
      return { kind: 'ready', home, character: home.character, creation, pack }
    }
    load().then(
      (next) => {
        if (live) setState(next)
      },
      () => {
        if (live) setState({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [campaignId])

  if (state.kind === 'ready') {
    if (state.character.status === 'submitted' || state.character.status === 'validated') {
      return <Navigate to={playPath(campaignId)} replace />
    }
    return <Creator campaignId={campaignId} {...state} />
  }
  return (
    <CreatorFrame>
      <Message state={state.kind} />
    </CreatorFrame>
  )
}

function Message({ state }: { state: 'loading' | 'not-joined' | 'error' }) {
  const { t } = useTranslation()
  if (state === 'loading') return <p role="status">{t('play.loading')}</p>
  if (state === 'not-joined') return <p role="alert">{t('play.notJoined')}</p>
  return <p role="alert">{t('play.error')}</p>
}

function CreatorFrame({ children }: { children: ReactNode }) {
  return <main className="surface-table mx-auto flex min-h-dvh max-w-md flex-col gap-3 p-4 text-chalk">{children}</main>
}

function Creator({
  campaignId,
  home,
  character: initial,
  creation,
  pack,
}: {
  campaignId: string
  home: PlayerHome
  character: CharacterView
  creation: Creation
  pack: PackCatalogue
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const rules = creation.rules
  const steps = useMemo(() => stepsFor(creation), [creation])
  const parts = useMemo(() => [...new Set(steps.map(partOf))], [steps])
  const [index, setIndex] = useState(0)
  const [sheet, setSheet] = useState<CharacterSheet>(() => {
    const s = initial.sheet
    const cls = rules?.classes.find((c) => c.id === s.classId)
    return {
      ...s,
      look: s.look ?? creation.startLook,
      abilities: s.abilities && Object.keys(s.abilities).length ? s.abilities : (cls?.abilities ?? {}),
    }
  })
  const [character, setCharacter] = useState(initial)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<ErrorKey | null>(null)
  const [sent, setSent] = useState(false)
  const [opened] = useState(() => Date.now())
  const [elapsed, setElapsed] = useState(0)

  const step = steps[index]
  const look = sheet.look as CharacterLook
  const cls = rules?.classes.find((c) => c.id === sheet.classId)
  const people = rules?.peoples.find((p) => p.id === sheet.peopleId)
  const over = cls ? overBudget(sheet.abilities ?? {}, cls.budget) : 0
  const gm = home.campaign.gmName
  const missing = (['name', 'people', 'class'] as const).filter((m) => {
    if (m === 'name') return !sheet.name?.trim()
    if (m === 'people') return Boolean(rules?.peoples.length) && !sheet.peopleId
    return Boolean(rules?.classes.length) && !sheet.classId
  })

  const update = (patch: Partial<CharacterSheet>) => setSheet((s) => ({ ...s, ...patch }))

  /** Save the draft; the server answers the character with its numbers. */
  async function save(): Promise<boolean> {
    setBusy(true)
    setError(null)
    try {
      setCharacter(await saveCharacter(campaignId, sheet))
      return true
    } catch (err) {
      setError(errorKey(err))
      return false
    } finally {
      setBusy(false)
    }
  }

  async function next() {
    if (!(await save())) return
    if (steps[index + 1] === 'review') setElapsed(Date.now() - opened)
    setIndex((i) => Math.min(i + 1, steps.length - 1))
    window.scrollTo?.(0, 0)
  }

  async function send() {
    if (!(await save())) return
    setBusy(true)
    try {
      setCharacter(await submitCharacter(campaignId))
      setSent(true)
    } catch (err) {
      setError(errorKey(err))
    } finally {
      setBusy(false)
    }
  }

  const subtitle = [people?.name, cls?.name, t('creator.review.level')].filter(Boolean).join(' · ')

  if (sent) {
    return (
      <CreatorFrame>
        <StatusBanner tone="wait">{t('creator.sent.banner', { gm })}</StatusBanner>
        <CharacterSummary sheet={sheet} look={look} subtitle={subtitle} stats={character.stats} over={over} gmName={gm} />
        <div className="cr-sent">
          <span className="cr-spin" aria-hidden />
          {t('creator.sent.waiting', { gm })}
        </div>
        <p className="text-center text-caption text-mute">{t('creator.sent.notify')}</p>
        <CardButton variant="dark" title={t('creator.sent.home')} onClick={() => navigate(playPath(campaignId))} />
      </CreatorFrame>
    )
  }

  const part = parts.indexOf(partOf(step)) + 1
  const lookTab = step === 'body' || step === 'outfit' || step === 'colours'

  return (
    <CreatorFrame>
      <StatusBanner tone={character.status === 'returned' ? 'warn' : 'you'}>
        {step === 'review' ? t('creator.banner.ready') : t('creator.banner.step', { part, total: parts.length })}
      </StatusBanner>
      {index > 0 && (
        <button type="button" className="self-start text-caption text-mute" onClick={() => setIndex(index - 1)}>
          {t('creator.back')}
        </button>
      )}
      <div className="cr-dots" aria-hidden>
        {parts.map((p, i) => (
          <i key={p} data-on={i < part || undefined} />
        ))}
      </div>
      {character.status === 'returned' && character.gmNote && (
        <div className="rounded-button bg-ivory px-3.5 py-3 text-body text-ink shadow-ivory-flat">
          <span className="type-label block text-ink-soft">{t('play.gmNote')}</span>
          {character.gmNote}
        </div>
      )}
      <h1 className="type-title text-[24px]">{t(`creator.titles.${step}`)}</h1>

      {step === 'people' && rules && (
        <OptionGrid options={rules.peoples} selected={sheet.peopleId} onPick={(id) => update({ peopleId: id })} />
      )}
      {lookTab && (
        <AppearanceStep
          pack={pack}
          look={look}
          onLook={(l) => update({ look: l })}
          tab={step}
          onTab={(tab) => setIndex(steps.indexOf(tab))}
          name={sheet.name ?? ''}
          onName={(name) => update({ name })}
        />
      )}
      {step === 'class' && rules && (
        <ClassStep
          classes={rules.classes}
          selected={sheet.classId}
          onPick={(id) =>
            update({ classId: id, abilities: { ...rules.classes.find((c) => c.id === id)?.abilities } })
          }
        />
      )}
      {step === 'abilities' && rules && cls && (
        <AbilitiesStep
          abilities={rules.abilities}
          scores={sheet.abilities ?? {}}
          defaults={cls.abilities}
          budget={cls.budget}
          className={cls.name}
          gmName={gm}
          onScore={(id, score) => update({ abilities: { ...sheet.abilities, [id]: score } })}
          onReset={() => update({ abilities: { ...cls.abilities } })}
        />
      )}
      {step === 'abilities' && !cls && <p className="text-body text-chalk-soft">{t('creator.abilities.noClass')}</p>}
      {step === 'story' && <StoryStep campaignId={campaignId} backstory={sheet.backstory ?? {}} onChange={(backstory) => update({ backstory })} />}
      {step === 'review' && (
        <>
          <CharacterSummary sheet={sheet} look={look} subtitle={subtitle} stats={character.stats} over={over} gmName={gm} />
          {missing.length > 0 && (
            <div className="cr-warn" role="note">
              <b>{t('creator.review.missingTitle')}</b>
              {missing.map((m) => t(`creator.review.missing.${m}`)).join(' · ')}
            </div>
          )}
        </>
      )}

      {error && (
        <p role="alert" className="text-body text-stat-hp">
          {t(`creator.errors.${error}`)}
        </p>
      )}
      <div className="mt-auto pt-2">
        {step === 'review' ? (
          <CardButton
            title={t('creator.send')}
            subtitle={t('creator.sendTime', { time: clock(elapsed) })}
            icon="send"
            disabled={busy || missing.length > 0}
            pressed={busy}
            onClick={send}
          />
        ) : (
          <CardButton
            title={t('creator.next')}
            subtitle={t(`creator.titles.${steps[index + 1]}`)}
            disabled={busy || (step === 'class' && !sheet.classId) || (step === 'people' && !sheet.peopleId)}
            pressed={busy}
            onClick={next}
          />
        )}
      </div>
    </CreatorFrame>
  )
}
