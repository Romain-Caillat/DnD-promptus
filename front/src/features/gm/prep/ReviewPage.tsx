import { useCallback, useEffect, useState } from 'react'
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
  nameOf,
  validateCampaign,
  type ActReadiness,
  type Edit,
  type ReviewCampaign,
  type Story,
  type StoryIssue,
} from '@/lib/prep'
import { cn } from '@/lib/utils'
import { EntityForm, type FieldDef } from './EntityForm'
import { Gauge, Readiness, useReadiness } from './Readiness'
import { SceneSheet } from './SceneSheet'
import { Workshop, useWorkshop } from './Workshop'

const TABS = ['graph', 'bible', 'sheets', 'coherence', 'validate'] as const
type Tab = (typeof TABS)[number]

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; campaign: ReviewCampaign }

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
  const [error, setError] = useState<{ code: string; detail: string | null } | null>(null)

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
  const acts = useReadiness(campaignId, state.kind === 'ready' ? state.campaign.updatedAt : '')

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
      setError(err instanceof ApiError ? { code: err.code, detail: err.detail } : { code: 'action', detail: null })
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
      setError(err instanceof ApiError ? { code: err.code, detail: err.detail } : { code: 'action', detail: null })
    } finally {
      setBusy(false)
    }
  }

  const selectedNode = story.nodes?.find((n) => n.id === selected)
  const sceneOpen = tab === 'graph' && Boolean(selectedNode)
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
            {t(`prep.review.errors.${error.code}`, { defaultValue: t('prep.review.errors.action') })}
            {error.code === 'EDIT_SCENE_INVALID' && error.detail && (
              <span className="block font-mono text-caption">{error.detail}</span>
            )}
          </p>
        )}
        <div
          className={cn(
            'grid gap-4',
            sceneOpen
              ? 'lg:grid-cols-[1fr_440px] xl:grid-cols-[300px_1fr_460px]'
              : 'xl:grid-cols-[340px_1fr_340px]',
          )}
        >
          {/* With a scene open on a tablet, the graph and its sheet side by side; the co-GM below. */}
          <div className={cn('flex min-w-0 flex-col', sceneOpen && 'lg:order-last lg:col-span-2 xl:order-none xl:col-span-1')}>
            <Workshop workshop={workshop} story={story} node={selectedNode?.id ?? null} />
          </div>

          <section className="flex min-w-0 flex-col gap-4">
            {tab === 'graph' && <Graph story={story} acts={acts} selected={selected} onSelect={setSelected} />}
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
            {tab === 'validate' && <Readiness acts={acts} />}
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
              <SceneSheet
                key={selectedNode.id}
                campaignId={campaignId}
                story={story}
                node={selectedNode}
                version={campaign.updatedAt}
                busy={busy}
                onSave={save}
              />
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
  acts: gauges,
  selected,
  onSelect,
}: {
  story: Story
  acts: ActReadiness[] | null
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
            {gauges
              ?.filter((g) => g.act === act.id)
              .map((g) => (
                <span key={g.act} className="flex flex-col gap-1 text-caption text-mute-soft">
                  {g.ready ? t('prep.readiness.ready') : t('prep.readiness.notReady', { done: g.done, total: g.total })}
                  <Gauge done={g.done} total={g.total} />
                </span>
              ))}
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
