import { useEffect, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Btn, Panel } from '@/features/gm/live/ui'
import { ApiError } from '@/lib/api'
import {
  fetchCampaign,
  listRulePresets,
  presetOf,
  saveCampaignSettings,
  type CampaignDetail,
  type RulePreset,
  type StoryAct,
  type StoryFront,
} from '@/lib/campaigns'
import { applyEdits, type ClaudeDecision, type Edit, type Proposal } from '@/lib/prep'
import { IdeaFields, PresetStats, TableFields } from '../CampaignFields'
import { bibleEditsOf, formOf, settingsChanged, settingsOf, type CampaignForm } from '../campaignForm'
import { TextField } from '../fields'
import { IDEA_TARGETS, adding, nextId, setting, shownValue, stepsOf, touching, waiting, type Waiting } from './idea'

/** What the GM's gestures on Claude's items need. */
interface ClaudeHands {
  batches: readonly Proposal[]
  busy: boolean
  onPick: (batch: string) => void
  onDecide: (p: Proposal, decision: ClaudeDecision) => void
}

type Notice = { kind: 'saved' } | { kind: 'error'; text: string }

/**
 * campaign/write-story-by-map-scale — « L'idée », in the Atlas's side
 * panel at the widest scale (alone when the campaign has no map yet):
 * the title, universe, the GM's idea, the players' hook, tone and
 * themes, then the acts and the fronts; the table's size, the AI budget
 * and the rule system fold below. The settings route saves what it
 * carries, story edits the rest — both journaled. Claude's waiting
 * items about any of it show dotted under the field they would change,
 * kept or discarded there; what someone else just changed is marked.
 */
export function IdeaSection({
  campaignId,
  campaign,
  fresh,
  claude,
  onSaved,
}: {
  campaignId: string
  campaign: CampaignDetail
  fresh: ReadonlySet<string>
  claude: ClaudeHands
  /** The campaign as it is after a save. */
  onSaved: (campaign: CampaignDetail) => void
}) {
  const { t } = useTranslation()
  /** What the GM typed; `null` while they type nothing, so a change by Claude shows at once. */
  const [draft, setDraft] = useState<CampaignForm | null>(null)
  const [busy, setBusy] = useState(false)
  const [notice, setNotice] = useState<Notice | null>(null)
  const [presets, setPresets] = useState<RulePreset[]>([])
  const [opened, setOpened] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    // Only names the rule system: « L'idée » works without it.
    listRulePresets().then(
      (list) => {
        if (live) setPresets(list)
      },
      () => {},
    )
    return () => {
      live = false
    }
  }, [])

  const form = draft ?? formOf(campaign)
  const preset = presetOf(presets, campaign.story.rules)
  const acts = campaign.story.acts ?? []
  const fronts = campaign.story.fronts ?? []

  const errorText = (err: unknown) => {
    if (err instanceof ApiError) {
      if (['TITLE_REQUIRED', 'INVALID_PLAYER_COUNT', 'INVALID_AI_BUDGET'].includes(err.code)) {
        return t(`gm.newCampaign.errors.${err.code as 'TITLE_REQUIRED'}`)
      }
      return t(`prep.review.errors.${err.code}`, { defaultValue: t('gm.campaign.actionError') })
    }
    return t('gm.campaign.actionError')
  }

  async function save() {
    const settings = settingsOf(form)
    if (typeof settings === 'string') {
      setNotice({ kind: 'error', text: t(`gm.newCampaign.errors.${settings}`) })
      return
    }
    const edits = bibleEditsOf(form, campaign)
    setBusy(true)
    setNotice(null)
    try {
      let updated = campaign
      if (settingsChanged(settings, campaign)) updated = await saveCampaignSettings(campaignId, settings)
      if (edits.length > 0) {
        await applyEdits(campaignId, edits)
        updated = await fetchCampaign(campaignId)
      }
      setDraft(null)
      setNotice({ kind: 'saved' })
      onSaved(updated)
    } catch (err) {
      setNotice({ kind: 'error', text: errorText(err) })
    } finally {
      setBusy(false)
    }
  }

  /** Story edits of an act or a front, then the campaign again; `false` when refused. */
  async function send(edits: Edit[]): Promise<boolean> {
    setBusy(true)
    setNotice(null)
    try {
      await applyEdits(campaignId, edits)
      onSaved(await fetchCampaign(campaignId))
      return true
    } catch (err) {
      setNotice({ kind: 'error', text: errorText(err) })
      return false
    } finally {
      setBusy(false)
    }
  }

  async function addAct() {
    const id = nextId('acte_', acts)
    if (await send([{ op: 'add', kind: 'act', value: { id, title: t('gm.idea.newAct', { n: acts.length + 1 }) } }])) {
      setOpened(id)
    }
  }

  async function addFront() {
    const id = nextId('front_', fronts)
    if (await send([{ op: 'add', kind: 'front', value: { id, name: t('gm.idea.newFront'), goal: '', steps: [] } }])) {
      setOpened(id)
    }
  }

  const change = (patch: Partial<CampaignForm>) => {
    setDraft({ ...form, ...patch })
    setNotice(null)
  }
  const below = (key: keyof CampaignForm) => {
    const at = IDEA_TARGETS[key]
    return at ? <Proposed items={waiting(claude.batches, setting(...at))} claude={claude} /> : null
  }
  const freshIdea = fresh.has('campaign') || fresh.has('bible')

  return (
    <Panel title={t('gm.idea.title')} actions={freshIdea ? <FreshMark /> : undefined}>
      <p className="text-caption text-mute-soft">{t('gm.idea.intro')}</p>
      <form
        className="flex flex-col gap-3"
        aria-label={t('gm.idea.title')}
        onSubmit={(e) => {
          e.preventDefault()
          void save()
        }}
      >
        <IdeaFields form={form} onChange={change} below={below} />
        <details className="flex flex-col gap-2">
          <summary className="cursor-pointer text-caption font-semibold text-chalk">{t('gm.idea.settings')}</summary>
          <div className="mt-2 flex flex-col gap-3">
            <TableFields form={form} onChange={change} />
            <div className="flex flex-col gap-1.5">
              <span className="text-caption text-mute-soft">{t('gm.campaign.rules')}</span>
              {preset ? (
                <>
                  <span className="type-title text-[14px]">{preset.name}</span>
                  <PresetStats preset={preset} />
                </>
              ) : (
                <span className="text-caption text-mute-soft">{t('gm.campaigns.rulesRef', { ...campaign.story.rules })}</span>
              )}
            </div>
          </div>
        </details>
        <div className="flex flex-wrap items-center justify-end gap-2">
          {notice?.kind === 'saved' && (
            <p role="status" className="text-caption text-chalk-soft">
              {t('gm.idea.saved')}
            </p>
          )}
          {notice?.kind === 'error' && (
            <p role="alert" className="text-caption text-stat-atk">
              {notice.text}
            </p>
          )}
          <Btn main type="submit" disabled={busy || draft === null}>
            {busy ? t('gm.idea.saving') : t('gm.idea.save')}
          </Btn>
        </div>
      </form>

      <section aria-label={t('gm.idea.acts', { count: acts.length })} className="flex flex-col gap-1.5">
        <h3 className="type-label text-mute-soft">{t('gm.idea.acts', { count: acts.length })}</h3>
        {acts.length === 0 && <p className="text-caption text-mute-soft">{t('gm.idea.noActs')}</p>}
        {acts.map((act) => (
          <ActEditor
            key={act.id}
            act={act}
            open={opened === act.id}
            fresh={fresh.has(act.id)}
            busy={busy}
            claude={claude}
            onSave={send}
          />
        ))}
        <Proposed items={waiting(claude.batches, adding('act'))} claude={claude} />
        <Btn className="self-start" disabled={busy} onClick={() => void addAct()}>
          {t('gm.idea.addAct')}
        </Btn>
      </section>

      <section aria-label={t('gm.idea.fronts', { count: fronts.length })} className="flex flex-col gap-1.5">
        <h3 className="type-label text-mute-soft">{t('gm.idea.fronts', { count: fronts.length })}</h3>
        {fronts.length === 0 && <p className="text-caption text-mute-soft">{t('gm.idea.noFronts')}</p>}
        {fronts.map((front) => (
          <FrontEditor
            key={front.id}
            front={front}
            open={opened === front.id}
            fresh={fresh.has(front.id)}
            busy={busy}
            claude={claude}
            onSave={send}
          />
        ))}
        <Proposed items={waiting(claude.batches, adding('front'))} claude={claude} />
        <Btn className="self-start" disabled={busy} onClick={() => void addFront()}>
          {t('gm.idea.addFront')}
        </Btn>
      </section>
    </Panel>
  )
}

/** The small mark on what someone else changed just now. */
function FreshMark() {
  const { t } = useTranslation()
  return <span className="type-key border border-dashed border-current px-1 text-label">{t('prep.changes.fresh')}</span>
}

/** Claude's waiting items about one field or list, dotted, each kept or discarded on the spot. */
function Proposed({ items, claude }: { items: Waiting[]; claude: ClaudeHands }) {
  const { t } = useTranslation()
  if (items.length === 0) return null
  return (
    <ul className="flex flex-col gap-1">
      {items.map(({ proposal, item }) => {
        const c = item.change
        const name = c.name || c.id
        const text =
          c.op === 'add'
            ? t('gm.idea.proposedAdd', { name })
            : c.op === 'remove'
              ? t('gm.idea.proposedRemove', { name })
              : shownValue(c.after)
                ? t('gm.idea.proposed', { value: shownValue(c.after) })
                : t('gm.idea.proposedClear')
        return (
          <li
            key={`${proposal.id}:${item.key}`}
            className="flex flex-col gap-1 border border-dashed border-line-strong p-1.5 text-caption text-chalk"
            data-proposal={proposal.id}
          >
            <button type="button" className="text-left" onClick={() => claude.onPick(proposal.id)}>
              {text}
            </button>
            {item.stale && <span className="text-stat-hp">{t('atlas.claude.stale')}</span>}
            <div className="flex flex-wrap gap-1.5">
              <Btn disabled={claude.busy || item.stale} onClick={() => claude.onDecide(proposal, { keep: [item.key] })}>
                {t('atlas.claude.keep')}
              </Btn>
              <Btn disabled={claude.busy} onClick={() => claude.onDecide(proposal, { discard: [item.key] })}>
                {t('atlas.claude.discard')}
              </Btn>
            </div>
          </li>
        )
      })}
    </ul>
  )
}

/** One act or front, folded under its name: its fields, Claude's items about it, its own save. */
function Folded({
  name,
  open,
  fresh,
  waits,
  children,
}: {
  name: string
  open: boolean
  fresh: boolean
  waits: number
  children: ReactNode
}) {
  return (
    <details className="border border-line px-2 py-1.5" open={open || undefined}>
      <summary className="flex cursor-pointer items-center gap-1.5 text-caption font-semibold">
        <span className="flex-1">{name}</span>
        {waits > 0 && <span aria-hidden="true">▶</span>}
        {fresh && <FreshMark />}
      </summary>
      <div className="mt-2 flex flex-col gap-2">{children}</div>
    </details>
  )
}

type Draft = Record<string, string>

/** The `set` edits of the fields of `draft` that differ from `base`; `required` ones are never cleared. */
function editsOf(target: string, base: Draft, draft: Draft, required: readonly string[]): Edit[] {
  return Object.keys(draft)
    .filter((k) => draft[k].trim() !== base[k])
    .map((k) => ({ op: 'set', target, field: k, value: draft[k].trim() || (required.includes(k) ? '' : null) }))
}

const ACT_FIELDS = [
  { key: 'title', multiline: false },
  { key: 'summary', multiline: true },
  { key: 'opening', multiline: true },
  { key: 'closing', multiline: true },
] as const

function ActEditor({
  act,
  open,
  fresh,
  busy,
  claude,
  onSave,
}: {
  act: StoryAct
  open: boolean
  fresh: boolean
  busy: boolean
  claude: ClaudeHands
  onSave: (edits: Edit[]) => Promise<boolean>
}) {
  const { t } = useTranslation()
  const base: Draft = Object.fromEntries(ACT_FIELDS.map((f) => [f.key, act[f.key] ?? '']))
  const [draft, setDraft] = useState<Draft | null>(null)
  const values = draft ?? base
  const edits = draft ? editsOf(act.id, base, draft, ['title']) : []
  const all = waiting(claude.batches, touching(act.id))
  return (
    <Folded name={act.title || act.id} open={open} fresh={fresh} waits={all.length}>
      <Proposed items={all.filter((w) => w.item.change.op === 'remove')} claude={claude} />
      {ACT_FIELDS.map((f) => (
        <div key={f.key} className="flex flex-col gap-1">
          <TextField
            label={t(`prep.review.field.${f.key}`)}
            multiline={f.multiline}
            value={values[f.key]}
            onChange={(v) => setDraft({ ...values, [f.key]: v })}
          />
          <Proposed items={waiting(claude.batches, setting(act.id, f.key))} claude={claude} />
        </div>
      ))}
      <Btn
        main
        className="self-end"
        disabled={busy || edits.length === 0 || !values.title.trim()}
        onClick={() => void onSave(edits).then((ok) => ok && setDraft(null))}
      >
        {t('prep.review.save')}
      </Btn>
    </Folded>
  )
}

const FRONT_FIELDS = [
  { key: 'name', multiline: false },
  { key: 'goal', multiline: true },
  { key: 'description', multiline: true },
] as const
/** The clock, one step per line. */
const CLOCK = { key: 'steps', multiline: true } as const

function FrontEditor({
  front,
  open,
  fresh,
  busy,
  claude,
  onSave,
}: {
  front: StoryFront
  open: boolean
  fresh: boolean
  busy: boolean
  claude: ClaudeHands
  onSave: (edits: Edit[]) => Promise<boolean>
}) {
  const { t } = useTranslation()
  const base: Draft = {
    ...Object.fromEntries(FRONT_FIELDS.map((f) => [f.key, front[f.key] ?? ''])),
    [CLOCK.key]: front.steps.map((s) => s.label).join('\n'),
  }
  const [draft, setDraft] = useState<Draft | null>(null)
  const values = draft ?? base
  const { [CLOCK.key]: clock, ...plain } = values
  const { [CLOCK.key]: baseClock, ...plainBase } = base
  const edits: Edit[] = draft ? editsOf(front.id, plainBase, plain, ['name', 'goal']) : []
  const steps = stepsOf(clock, front.steps)
  if (draft && steps.map((s) => s.label).join('\n') !== baseClock) {
    edits.push({ op: 'set', target: front.id, field: CLOCK.key, value: steps })
  }
  const all = waiting(claude.batches, touching(front.id))
  return (
    <Folded name={front.name || front.id} open={open} fresh={fresh} waits={all.length}>
      <Proposed items={all.filter((w) => w.item.change.op === 'remove')} claude={claude} />
      {[...FRONT_FIELDS, CLOCK].map((f) => (
        <div key={f.key} className="flex flex-col gap-1">
          <TextField
            label={t(`prep.review.field.${f.key}`)}
            multiline={f.multiline}
            value={values[f.key]}
            onChange={(v) => setDraft({ ...values, [f.key]: v })}
          />
          <Proposed items={waiting(claude.batches, setting(front.id, f.key))} claude={claude} />
        </div>
      ))}
      <Btn
        main
        className="self-end"
        disabled={busy || edits.length === 0 || !values.name.trim()}
        onClick={() => void onSave(edits).then((ok) => ok && setDraft(null))}
      >
        {t('prep.review.save')}
      </Btn>
    </Folded>
  )
}
