import { useCallback, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useNavigate, useParams } from 'react-router'
import { CardButton } from '@/components/game/CardButton'
import { StatusBanner } from '@/components/game/StatusBanner'
import { Btn, Panel } from '@/features/gm/live/ui'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { ApiError } from '@/lib/api'
import {
  applyEdits,
  entityAtPath,
  fetchReview,
  freshId,
  nameOf,
  validateCampaign,
  valueAt,
  type Edit,
  type Entity,
  type ReviewCampaign,
  type Story,
  type StoryIssue,
} from '@/lib/prep'
import { cn } from '@/lib/utils'
import { TextField } from '../fields'
import { Workshop, useWorkshop } from './Workshop'

const TABS = ['graph', 'bible', 'sheets', 'coherence', 'validate'] as const
type Tab = (typeof TABS)[number]

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; campaign: ReviewCampaign }

/** A field of an entity form: its dotted path, and how it is written. */
interface FieldDef {
  path: string
  multiline?: boolean
  number?: boolean
  /** A list of lines (`truths`), edited one per line. */
  lines?: boolean
}

const NODE_FIELDS: FieldDef[] = [
  { path: 'title' },
  { path: 'summary', multiline: true },
  { path: 'read_aloud', multiline: true },
  { path: 'flow', multiline: true },
  { path: 'gm_notes', multiline: true },
]
const BIBLE_FIELDS: FieldDef[] = [
  { path: 'pitch', multiline: true },
  { path: 'tone' },
  { path: 'player_hook', multiline: true },
  { path: 'truths', lines: true },
  { path: 'secrets', lines: true },
  { path: 'art_direction', multiline: true },
]
const SHEET_FIELDS: Record<SheetKind, FieldDef[]> = {
  npcs: [
    { path: 'name' },
    { path: 'title' },
    { path: 'appearance', multiline: true },
    { path: 'roleplay', multiline: true },
    { path: 'motivation', multiline: true },
    { path: 'wants', multiline: true },
    { path: 'hides', multiline: true },
    { path: 'gm_notes', multiline: true },
  ],
  adversaries: [
    { path: 'name' },
    { path: 'description', multiline: true },
    { path: 'stats.hit_points', number: true },
    { path: 'stats.armor_class', number: true },
    { path: 'gm_notes', multiline: true },
  ],
  items: [
    { path: 'name' },
    { path: 'description', multiline: true },
    { path: 'effect', multiline: true },
    { path: 'gm_notes', multiline: true },
  ],
}
const SHEET_KINDS = ['npcs', 'adversaries', 'items'] as const
type SheetKind = (typeof SHEET_KINDS)[number]

/**
 * `/campagnes/:campaignId/preparer` — the GM reviews a campaign
 * (campaign/review-story-graph, planche « Préparer », moments 6 to 8
 * and 10): the graph of scenes and clues, the bible, the sheets, the
 * coherence alerts, and the validation that makes it playable. The
 * co-GM's workshop stays on the left: it proposes, the GM accepts or
 * rejects.
 */
export function ReviewPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [tab, setTab] = useState<Tab>('graph')
  const [selected, setSelected] = useState<string | null>(null)
  const [version, setVersion] = useState(0)
  const [workshopVersion, setWorkshopVersion] = useState(0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    fetchReview(campaignId).then(
      (campaign) => {
        if (live) setState({ kind: 'ready', campaign })
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
    if (topics.includes('story')) setVersion((v) => v + 1)
    if (topics.includes('desk')) setWorkshopVersion((v) => v + 1)
  })

  const adopt = useCallback((campaign: ReviewCampaign) => setState({ kind: 'ready', campaign }), [])
  const workshop = useWorkshop(campaignId, workshopVersion, adopt)

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />
  if (state.kind !== 'ready') {
    return (
      <main className="surface-table flex min-h-dvh flex-col gap-4 p-6 text-chalk">
        <Link className="text-caption text-mute-soft underline underline-offset-4" to={`/campagnes/${campaignId}`}>
          {t('prep.back')}
        </Link>
        {state.kind === 'loading' && <p role="status">{t('prep.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('prep.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('prep.error')}</p>}
      </main>
    )
  }

  const { campaign } = state
  const story = campaign.story

  async function save(edits: Edit[]) {
    if (edits.length === 0) return
    setBusy(true)
    setError(null)
    try {
      adopt((await applyEdits(campaignId, edits)).campaign)
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'action')
    } finally {
      setBusy(false)
    }
  }

  async function validate() {
    setBusy(true)
    setError(null)
    try {
      adopt(await validateCampaign(campaignId))
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'action')
    } finally {
      setBusy(false)
    }
  }

  const selectedNode = story.nodes?.find((n) => n.id === selected)
  const selectedSheet = SHEET_KINDS.flatMap((k) => (story[k] ?? []).map((e) => ({ kind: k, entity: e }))).find(
    (s) => s.entity.id === selected,
  )

  return (
    <main className="surface-table flex min-h-dvh flex-col text-chalk">
      <header className="flex flex-wrap items-center gap-4 border-b border-line px-5 py-3 text-caption text-mute-soft">
        <Link className="underline underline-offset-4" to={`/campagnes/${campaignId}`}>
          {t('prep.back')}
        </Link>
        <h1 className="type-title text-[15px] text-chalk">{story.title}</h1>
        <span className="rounded-md border border-line-strong px-2 py-0.5 text-[11px] font-semibold">
          {campaign.validatedAt ? t('prep.review.validated') : t('prep.review.notValidated')}
        </span>
        <nav className="ml-auto flex gap-1" aria-label={t('prep.review.tabs')}>
          {TABS.map((id) => (
            <button
              key={id}
              type="button"
              aria-current={tab === id ? 'page' : undefined}
              className={cn(
                'rounded-button px-2.5 py-1.5 text-caption font-semibold',
                tab === id ? 'bg-ivory text-ink' : 'text-chalk hover:bg-surface',
              )}
              onClick={() => setTab(id)}
            >
              {id === 'coherence'
                ? t('prep.review.tab.coherence', { count: campaign.issues.length })
                : t(`prep.review.tab.${id}`)}
            </button>
          ))}
        </nav>
      </header>

      <div className="flex flex-col gap-4 p-5">
        {error && (
          <p role="alert" className="text-body text-stat-atk">
            {t(`prep.review.errors.${error}`, { defaultValue: t('prep.review.errors.action') })}
          </p>
        )}
        <div className="grid gap-4 xl:grid-cols-[340px_1fr_340px]">
          <Workshop workshop={workshop} story={story} node={selectedNode?.id ?? null} />

          <section className="flex min-w-0 flex-col gap-4">
            {tab === 'graph' && <Graph story={story} selected={selected} onSelect={setSelected} />}
            {tab === 'bible' && (
              <EntityForm
                key={`bible${campaign.updatedAt}`}
                title={t('prep.review.tab.bible')}
                target="bible"
                entity={story.bible}
                fields={BIBLE_FIELDS}
                busy={busy}
                onSave={save}
              />
            )}
            {tab === 'sheets' && <Sheets story={story} selected={selected} onSelect={setSelected} />}
            {tab === 'coherence' && (
              <Coherence
                story={story}
                issues={campaign.issues}
                onAsk={(issue) => void workshop.send({ issue: { code: issue.code, path: issue.path } })}
              />
            )}
            {tab === 'validate' && (
              <Validate
                campaign={campaign}
                busy={busy}
                onValidate={() => void validate()}
                onInvite={() => navigate(`/campagnes/${campaignId}/table`)}
              />
            )}
          </section>

          <aside className="flex flex-col gap-4">
            {tab === 'graph' && selectedNode && (
              <NodePanel key={`${selectedNode.id}${campaign.updatedAt}`} story={story} node={selectedNode} busy={busy} onSave={save} />
            )}
            {tab === 'sheets' && selectedSheet && (
              <EntityForm
                key={`${selectedSheet.entity.id}${campaign.updatedAt}`}
                title={nameOf(selectedSheet.entity)}
                target={selectedSheet.entity.id}
                entity={selectedSheet.entity}
                fields={SHEET_FIELDS[selectedSheet.kind]}
                busy={busy}
                onSave={save}
              />
            )}
          </aside>
        </div>
      </div>
    </main>
  )
}

function Graph({
  story,
  selected,
  onSelect,
}: {
  story: Story
  selected: string | null
  onSelect: (id: string) => void
}) {
  const { t } = useTranslation()
  const acts = story.acts ?? []
  const nodes = story.nodes ?? []
  const clueCount = (node: string) => (story.clues ?? []).filter((c) => c.node === node).length
  return (
    <Panel title={t('prep.review.graphTitle', { count: nodes.length, acts: acts.length })}>
      {nodes.length === 0 && <p className="text-body text-chalk-soft">{t('prep.review.noScene')}</p>}
      <div className="grid gap-3 md:grid-cols-3">
        {acts.map((act) => (
          <div key={act.id} className="flex flex-col gap-2">
            <h3 className="type-label text-mute-soft">{act.title}</h3>
            {nodes
              .filter((n) => n.act === act.id)
              .map((n) => (
                <button
                  key={n.id}
                  type="button"
                  aria-pressed={selected === n.id}
                  className={cn(
                    'flex flex-col gap-0.5 rounded-xl p-2.5 text-left',
                    selected === n.id ? 'bg-ivory text-ink shadow-ivory-flat' : 'border border-line bg-surface',
                  )}
                  onClick={() => onSelect(n.id)}
                >
                  <span className="text-[11px] font-semibold uppercase tracking-wide opacity-70">
                    {n.optional ? t('prep.review.optional') : t('prep.review.scene')}
                  </span>
                  <b className="text-body">{n.title}</b>
                  <span className="text-caption opacity-70">{t('prep.review.clues', { count: clueCount(n.id) })}</span>
                </button>
              ))}
          </div>
        ))}
      </div>
    </Panel>
  )
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

/** One entity's fields, saved as `set` edits of the changed ones. */
function EntityForm({
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
  children?: React.ReactNode
}) {
  const { t } = useTranslation()
  const initial = useMemo(
    () => Object.fromEntries(fields.map((f) => [f.path, shown(valueAt(entity, f.path), f)])),
    [entity, fields],
  )
  const [values, setValues] = useState(initial)
  const changed = fields.filter((f) => values[f.path] !== initial[f.path])
  return (
    <Panel title={title}>
      {fields.map((f) => (
        <TextField
          key={f.path}
          label={t(`prep.review.field.${f.path}`, { defaultValue: f.path })}
          multiline={f.multiline || f.lines}
          value={values[f.path] ?? ''}
          onChange={(v) => setValues((cur) => ({ ...cur, [f.path]: v }))}
        />
      ))}
      <Btn
        main
        className="self-end"
        disabled={busy || changed.length === 0}
        onClick={() =>
          void onSave(changed.map((f) => ({ op: 'set', target, field: f.path, value: stored(values[f.path], f) })))
        }
      >
        {t('prep.review.save')}
      </Btn>
      {children}
    </Panel>
  )
}

function NodePanel({
  story,
  node,
  busy,
  onSave,
}: {
  story: Story
  node: NonNullable<Story['nodes']>[number]
  busy: boolean
  onSave: (edits: Edit[]) => Promise<void>
}) {
  const { t } = useTranslation()
  const clues = (story.clues ?? []).filter((c) => c.node === node.id)
  const revelations = story.revelations ?? []
  const [revelation, setRevelation] = useState(revelations[0]?.id ?? '')
  const [text, setText] = useState('')
  return (
    <>
      <EntityForm
        title={node.title}
        target={node.id}
        entity={node}
        fields={NODE_FIELDS}
        busy={busy}
        onSave={onSave}
      />
      <Panel title={t('prep.review.cluesTitle')}>
        {clues.length === 0 && <p className="text-body text-chalk-soft">{t('prep.review.noClue')}</p>}
        <ul className="flex flex-col gap-1.5">
          {clues.map((c) => (
            <li key={c.id} className="flex flex-col rounded-lg bg-surface px-2.5 py-1.5 text-caption">
              <b className="text-chalk">{c.text}</b>
              <span className="text-mute-soft">
                → {nameOf(revelations.find((r) => r.id === c.revelation))}
              </span>
            </li>
          ))}
        </ul>
        {revelations.length > 0 && (
          <div className="flex flex-col gap-2 border-t border-line pt-2">
            <label className="flex flex-col gap-1 text-caption text-mute-soft">
              {t('prep.review.clueLeadsTo')}
              <select
                className="rounded-button border border-line bg-table px-2 py-1.5 text-body text-chalk"
                value={revelation}
                onChange={(e) => setRevelation(e.target.value)}
              >
                {revelations.map((r) => (
                  <option key={r.id} value={r.id}>
                    {r.statement}
                  </option>
                ))}
              </select>
            </label>
            <TextField label={t('prep.review.clueText')} multiline value={text} onChange={setText} />
            <Btn
              main
              className="self-end"
              disabled={busy || !text.trim() || !revelation}
              onClick={() => {
                const value: Entity = {
                  id: freshId('cl_', text, story),
                  revelation,
                  node: node.id,
                  text: text.trim(),
                }
                void onSave([{ op: 'add', kind: 'clue', value }]).then(() => setText(''))
              }}
            >
              {t('prep.review.addClue')}
            </Btn>
          </div>
        )}
      </Panel>
    </>
  )
}

function Sheets({
  story,
  selected,
  onSelect,
}: {
  story: Story
  selected: string | null
  onSelect: (id: string) => void
}) {
  const { t } = useTranslation()
  return (
    <Panel title={t('prep.review.tab.sheets')}>
      {SHEET_KINDS.map((kind) => (
        <div key={kind} className="flex flex-col gap-1.5">
          <h3 className="type-label text-mute-soft">
            {t(`prep.review.sheetKind.${kind}`, { count: (story[kind] ?? []).length })}
          </h3>
          <div className="flex flex-wrap gap-2">
            {(story[kind] ?? []).map((e) => (
              <button
                key={e.id}
                type="button"
                aria-pressed={selected === e.id}
                className={cn(
                  'rounded-button px-2.5 py-1.5 text-caption font-semibold',
                  selected === e.id ? 'bg-ivory text-ink' : 'border border-line text-chalk',
                )}
                onClick={() => onSelect(e.id)}
              >
                {nameOf(e)}
              </button>
            ))}
          </div>
        </div>
      ))}
    </Panel>
  )
}

function Coherence({
  story,
  issues,
  onAsk,
}: {
  story: Story
  issues: StoryIssue[]
  onAsk: (issue: StoryIssue) => void
}) {
  const { t } = useTranslation()
  return (
    <Panel title={t('prep.review.coherenceTitle')}>
      <p className="text-caption text-mute-soft">{t('prep.review.coherenceHint')}</p>
      {issues.length === 0 && <StatusBanner tone="listen">{t('prep.review.coherent')}</StatusBanner>}
      <ul className="flex flex-col gap-2">
        {issues.map((i) => {
          const where = entityAtPath(story, i.path)
          return (
            <li
              key={`${i.code}${i.path}${i.detail}`}
              className={cn(
                'flex flex-col gap-1 rounded-xl border p-2.5',
                i.severity === 'error' ? 'border-stat-atk' : 'border-line',
              )}
            >
              <b className="text-body text-chalk">
                {t(`prep.review.codes.${i.code}`, { defaultValue: i.detail })}
              </b>
              <span className="text-caption text-mute-soft">
                {t(`prep.review.severity.${i.severity}`)}
                {where && ` · ${nameOf(where)}`}
              </span>
              <span className="font-mono text-[11px] text-mute-soft">{i.detail}</span>
              <Btn className="self-end" onClick={() => onAsk(i)}>
                {t('prep.review.askFix')}
              </Btn>
            </li>
          )
        })}
      </ul>
    </Panel>
  )
}

function Validate({
  campaign,
  busy,
  onValidate,
  onInvite,
}: {
  campaign: ReviewCampaign
  busy: boolean
  onValidate: () => void
  onInvite: () => void
}) {
  const { t } = useTranslation()
  const s = campaign.story
  const errors = campaign.issues.filter((i) => i.severity === 'error').length
  const counts: ['scenes' | 'acts' | 'revelations' | 'sheets' | 'fronts', number][] = [
    ['scenes', s.nodes?.length ?? 0],
    ['acts', s.acts?.length ?? 0],
    ['revelations', s.revelations?.length ?? 0],
    ['sheets', (s.npcs?.length ?? 0) + (s.adversaries?.length ?? 0) + (s.items?.length ?? 0)],
    ['fronts', s.fronts?.length ?? 0],
  ]
  return (
    <Panel title={t('prep.review.validateTitle')}>
      <ul className="grid grid-cols-2 gap-2 md:grid-cols-5">
        {counts.map(([key, n]) => (
          <li key={key} className="flex flex-col rounded-xl bg-surface p-2.5">
            <b className="type-title text-[22px]">{n}</b>
            <span className="text-caption text-mute-soft">{t(`prep.review.count.${key}`, { count: n })}</span>
          </li>
        ))}
      </ul>
      <p className="text-body text-chalk-soft">
        {t('prep.review.issuesLeft', { errors, warnings: campaign.issues.length - errors })}
      </p>
      {campaign.validatedAt ? (
        <>
          <StatusBanner tone="listen">
            {t('prep.review.validatedOn', {
              date: new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long' }).format(new Date(campaign.validatedAt)),
            })}
          </StatusBanner>
          <CardButton icon="link" title={t('prep.review.invite')} subtitle={t('prep.review.inviteSub')} onClick={onInvite} />
        </>
      ) : (
        <CardButton
          title={t('prep.review.validate')}
          subtitle={errors > 0 ? t('prep.review.validateBlocked', { count: errors }) : t('prep.review.validateSub')}
          disabled={busy || errors > 0}
          onClick={onValidate}
        />
      )}
    </Panel>
  )
}
