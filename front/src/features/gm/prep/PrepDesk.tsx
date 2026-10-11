import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { CardButton } from '@/components/game/CardButton'
import { StatusBanner } from '@/components/game/StatusBanner'
import { Btn, Panel } from '@/features/gm/live/ui'
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
import { useClaudeWaiting } from './ClaudeBanner'
import { Workshop, useWorkshop } from './Workshop'

/** The drawer's preparation tabs, in the order the Atlas lists them. */
export const PREP_TABS = ['graph', 'bible', 'sheets', 'coherence', 'validate'] as const
export type PrepTab = (typeof PREP_TABS)[number]

type PageState =
  | { kind: 'loading' }
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
 * The Atlas's drawer for what has no place on a map (campaign/review-
 * story-graph, planche « Préparer », moments 6 to 8 and 10, now inside
 * gm/prep-on-atlas): the graph of scenes and clues with the scene sheet,
 * the bible, the sheets, the coherence alerts, and the validation that
 * makes the campaign playable. The co-GM's workshop stays on the left: it
 * proposes, the GM accepts or rejects.
 *
 * The Atlas owns the address (`?onglet=`, `?scene=`), the live channel
 * and Claude's banner: `live` moves with the story, `proposals` with the
 * co-GM's and Claude's batches, `fresh` names what someone else just
 * changed.
 */
export function PrepDesk({
  campaignId,
  tab,
  scene: selected,
  live,
  proposals,
  fresh,
  onScene: setSelected,
  onMap,
}: {
  campaignId: string
  tab: PrepTab
  scene: string | null
  live: number
  proposals: number
  fresh: ReadonlySet<string>
  onScene: (id: string | null) => void
  /** Back to the map (Claude's batches are kept there). */
  onMap: () => void
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<{ code: string; detail: string | null } | null>(null)

  useEffect(() => {
    let on = true
    fetchReview(campaignId).then(
      (campaign) => {
        if (on) setState({ kind: 'ready', campaign })
      },
      (err: unknown) => {
        if (!on) return
        if (err instanceof ApiError && err.status === 404) setState({ kind: 'not-found' })
        else setState({ kind: 'error' })
      },
    )
    return () => {
      on = false
    }
  }, [campaignId, live])

  const adopt = useCallback((campaign: ReviewCampaign) => setState({ kind: 'ready', campaign }), [])
  const workshop = useWorkshop(campaignId, proposals, adopt)
  const claudeWaiting = useClaudeWaiting(campaignId, proposals)
  const acts = useReadiness(campaignId, state.kind === 'ready' ? state.campaign.updatedAt : '')

  if (state.kind !== 'ready') {
    return (
      <div className="flex flex-col gap-4">
        {state.kind === 'loading' && <p role="status">{t('prep.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('prep.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('prep.error')}</p>}
      </div>
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
    <div className="flex flex-col gap-4">
      <p className="flex flex-wrap items-center gap-2 text-caption text-mute-soft">
        <span className="rounded-md border border-line-strong px-2 py-0.5 text-[11px] font-semibold">
          {campaign.validatedAt ? t('prep.review.validated') : t('prep.review.notValidated')}
        </span>
        {t('prep.review.tab.coherence', { count: campaign.issues.length })}
      </p>
      {claudeWaiting > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <StatusBanner tone="wait" className="flex-1">
            {t('prep.claude.waiting', { count: claudeWaiting })}
          </StatusBanner>
          <Btn onClick={onMap}>{t('prep.claude.toAtlas')}</Btn>
        </div>
      )}
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
          {tab === 'graph' && (
            <Graph story={story} acts={acts} selected={selected} fresh={fresh} onSelect={setSelected} />
          )}
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
          {tab === 'sheets' && (
            <Sheets story={story} selected={selected} fresh={fresh} onSelect={setSelected} />
          )}
          {tab === 'coherence' && (
            <Coherence
              story={story}
              issues={campaign.issues}
              onMap={onMap}
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
  )
}

/** The small mark on what someone else changed just now. */
function FreshMark() {
  const { t } = useTranslation()
  return <span className="type-key self-start border border-dashed border-current px-1 text-label">{t('prep.changes.fresh')}</span>
}

function Graph({
  story,
  acts: gauges,
  selected,
  fresh,
  onSelect,
}: {
  story: Story
  acts: ActReadiness[] | null
  selected: string | null
  fresh: ReadonlySet<string>
  onSelect: (id: string) => void
}) {
  const { t } = useTranslation()
  const acts = story.acts ?? []
  const nodes = story.nodes ?? []
  const clueCount = (node: string) => (story.clues ?? []).filter((c) => c.node === node).length
  // A scene is lit when it, or one of its clues, just changed.
  const isFresh = (node: string) => fresh.has(node)
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
                    'pixel-choice flex flex-col gap-0.5 py-2.5 pr-2.5 pb-3 text-left',
                  )}
                  onClick={() => onSelect(n.id)}
                >
                  <span className="type-key text-label opacity-70">
                    {n.optional ? t('prep.review.optional') : t('prep.review.scene')}
                  </span>
                  <b className="text-body">{n.title}</b>
                  <span className="text-caption opacity-70">{t('prep.review.clues', { count: clueCount(n.id) })}</span>
                  {isFresh(n.id) && <FreshMark />}
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
  fresh,
  onSelect,
}: {
  story: Story
  selected: string | null
  fresh: ReadonlySet<string>
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
                  'pixel-choice flex items-center gap-1.5 py-1.5 pr-2.5 pb-2 text-caption font-semibold',
                )}
                onClick={() => onSelect(e.id)}
              >
                {nameOf(e)}
                {fresh.has(e.id) && <FreshMark />}
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
  onMap,
  onAsk,
}: {
  story: Story
  issues: StoryIssue[]
  /** Back to the map, where a scene is pinned. */
  onMap: () => void
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
              {i.code === 'SCENE_WITHOUT_PLACE' ? (
                // A spot on a map is pinned in the Atlas, never asked of the co-GM.
                <Btn className="self-end" onClick={onMap}>
                  {t('prep.review.pinInAtlas')}
                </Btn>
              ) : (
                <Btn className="self-end" onClick={() => onAsk(i)}>
                  {t('prep.review.askFix')}
                </Btn>
              )}
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
            <b className="type-title text-[24px]">{n}</b>
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
