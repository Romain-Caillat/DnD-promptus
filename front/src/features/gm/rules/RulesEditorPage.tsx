import { useCallback, useEffect, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useParams } from 'react-router'
import { CardButton } from '@/components/game/CardButton'
import { StatusBanner } from '@/components/game/StatusBanner'
import { Btn, Panel, field } from '@/features/gm/live/ui'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { ApiError } from '@/lib/api'
import {
  compareRules,
  discardRuleDraft,
  fetchRuleEditor,
  listOf,
  lockRuleDraft,
  saveRuleDraft,
  setAt,
  slugId,
  startRuleDraft,
  type Comparison,
  type RuleDocument,
  type RuleEditor,
  type RuleReport,
} from '@/lib/ruleEditor'
import { cn } from '@/lib/utils'
import { NumberField, TextField } from '../fields'
import { RuleChangeLine } from '@/components/game/RuleChangeLine'

const TABS = ['preset', 'stats', 'rolls', 'actions', 'house', 'creation', 'test', 'text', 'history'] as const
type Tab = (typeof TABS)[number]
const DERIVED = ['hit_points', 'armor_class'] as const
const BANDS = ['critical_failure', 'failure', 'success', 'critical_success'] as const
const DAMAGE = 'damage'
const COOLDOWN = 'cooldown'

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; editor: RuleEditor }

interface Named {
  id: string
  name: string
  description?: string
}

/**
 * `/campagnes/:campaignId/regles` — the rule system editor
 * (campaign/edit-rule-system, planche « Règles »): the first change
 * makes a draft of the version the campaign plays; every save shows what
 * the draft touches (what players will read, the scenes it breaks, the
 * lint, the fights replayed on both versions); locking makes it the
 * version of the next session. The structured tabs edit the draft as a
 * document; « Texte » edits the YAML itself.
 */
export function RulesEditorPage() {
  const { t } = useTranslation()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [tab, setTab] = useState<Tab>('preset')
  const [doc, setDoc] = useState<RuleDocument | null>(null)
  const [yaml, setYaml] = useState('')
  const [note, setNote] = useState('')
  const [dirty, setDirty] = useState<'doc' | 'yaml' | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<{ code: string; detail?: string } | null>(null)

  const adopt = useCallback((editor: RuleEditor) => {
    setState({ kind: 'ready', editor })
    setDoc(editor.draft?.document ?? null)
    setYaml(editor.draft?.yaml ?? '')
    setNote(editor.draft?.note ?? '')
    setDirty(null)
  }, [])

  const [version, setVersion] = useState(0)

  useEffect(() => {
    let live = true
    fetchRuleEditor(campaignId).then(
      (editor) => {
        if (live) adopt(editor)
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
  }, [adopt, campaignId, version])
  // Another tab of the GM saved the draft: reload unless this one has edits.
  useLiveChanges(campaignId, (topics) => {
    if (topics.includes('desk') && !dirty) setVersion((v) => v + 1)
  })

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

  const { editor } = state
  const draft = editor.draft

  async function run(action: () => Promise<RuleEditor>) {
    setBusy(true)
    setError(null)
    try {
      adopt(await action())
    } catch (err) {
      if (err instanceof ApiError) setError({ code: err.code })
      else setError({ code: 'action' })
    } finally {
      setBusy(false)
    }
  }

  async function save() {
    if (!draft) return
    setBusy(true)
    setError(null)
    try {
      const input = dirty === 'yaml' || !doc ? { yaml, note } : { document: doc, note }
      adopt(await saveRuleDraft(campaignId, input))
    } catch (err) {
      setError(err instanceof ApiError ? { code: err.code } : { code: 'action' })
    } finally {
      setBusy(false)
    }
  }

  const edit = (path: (string | number)[], value: unknown) => {
    if (!doc) return
    setDoc(setAt(doc, path, value))
    setDirty('doc')
  }

  return (
    <main className="surface-table flex min-h-dvh flex-col text-chalk">
      <header className="flex flex-wrap items-center gap-4 border-b border-line px-5 py-3 text-caption text-mute-soft">
        <Link className="underline underline-offset-4" to={`/campagnes/${campaignId}`}>
          {t('prep.back')}
        </Link>
        <h1 className="type-title text-[15px] text-chalk">{t('prep.rules.title', { name: editor.name })}</h1>
        <span className="rounded-md border border-line-strong px-2 py-0.5 text-[11px] font-semibold">
          {draft
            ? t('prep.rules.draftBadge', { version: draft.version })
            : t('prep.rules.versionBadge', { version: editor.current })}
        </span>
        {dirty && <span className="text-stat-init">{t('prep.rules.unsaved')}</span>}
      </header>

      <div className="flex flex-col gap-4 p-5">
        {editor.next !== null && (
          <StatusBanner tone="listen">{t('prep.rules.nextBanner', { version: editor.next })}</StatusBanner>
        )}
        {error && (
          <p role="alert" className="text-body text-stat-atk">
            {t(`prep.rules.errors.${error.code}`, { defaultValue: t('prep.rules.errors.action') })}
          </p>
        )}

        <div className="grid gap-4 lg:grid-cols-[200px_1fr_320px]">
          <nav className="surface-slab flex flex-col gap-1 p-2" aria-label={t('prep.rules.tabs')}>
            {TABS.map((id) => (
              <button
                key={id}
                type="button"
                aria-current={tab === id ? 'page' : undefined}
                className={cn(
                  'rounded-button px-2.5 py-2 text-left text-body',
                  tab === id ? 'bg-ivory text-ink' : 'text-chalk hover:bg-surface',
                )}
                onClick={() => setTab(id)}
              >
                {t(`prep.rules.tab.${id}`)}
              </button>
            ))}
          </nav>

          <section className="flex min-w-0 flex-col gap-4">
            {tab === 'preset' && (
              <PresetTab editor={editor} busy={busy} onStart={() => void run(() => startRuleDraft(campaignId))} />
            )}
            {tab === 'history' && <HistoryTab editor={editor} campaignId={campaignId} />}
            {tab !== 'preset' && tab !== 'history' && !draft && (
              <Panel title={t(`prep.rules.tab.${tab}`)}>
                <p className="text-body text-chalk-soft">{t('prep.rules.noDraft')}</p>
                <CardButton
                  title={t('prep.rules.start')}
                  subtitle={t('prep.rules.startSub', { version: editor.current })}
                  disabled={busy}
                  onClick={() => void run(() => startRuleDraft(campaignId))}
                />
              </Panel>
            )}
            {draft && doc && tab === 'stats' && <StatsTab doc={doc} edit={edit} />}
            {draft && doc && tab === 'rolls' && <RollsTab doc={doc} edit={edit} />}
            {draft && doc && tab === 'actions' && <ActionsTab doc={doc} edit={edit} />}
            {draft && doc && tab === 'house' && <HouseTab doc={doc} edit={edit} />}
            {draft && doc && tab === 'creation' && <CreationTab doc={doc} edit={edit} />}
            {draft && tab === 'test' && <TestTab report={draft.report} />}
            {draft && tab === 'text' && (
              <Panel title={t('prep.rules.tab.text')}>
                <p className="text-caption text-mute-soft">{t('prep.rules.textHint')}</p>
                <textarea
                  aria-label={t('prep.rules.tab.text')}
                  className={cn(field, 'min-h-[60vh] font-mono text-[12px]')}
                  value={yaml}
                  spellCheck={false}
                  onChange={(e) => {
                    setYaml(e.target.value)
                    setDirty('yaml')
                  }}
                />
              </Panel>
            )}
          </section>

          <aside className="flex flex-col gap-4">
            {draft ? (
              <>
                <Touches report={draft.report} onOpen={() => setTab('test')} />
                <Panel title={t('prep.rules.noteTitle')}>
                  <textarea
                    aria-label={t('prep.rules.noteTitle')}
                    className={cn(field, 'min-h-20')}
                    placeholder={t('prep.rules.notePlaceholder')}
                    value={note}
                    onChange={(e) => {
                      setNote(e.target.value)
                      setDirty((d) => d ?? 'doc')
                    }}
                  />
                </Panel>
                <CardButton
                  title={busy ? t('prep.rules.saving') : t('prep.rules.save')}
                  subtitle={t('prep.rules.saveSub')}
                  disabled={busy || !dirty}
                  onClick={() => void save()}
                />
                <CardButton
                  variant="dark"
                  title={t('prep.rules.lock', { version: draft.version })}
                  subtitle={t('prep.rules.lockSub')}
                  disabled={busy || dirty !== null}
                  onClick={() => void run(() => lockRuleDraft(campaignId))}
                />
                <Btn disabled={busy} onClick={() => void run(() => discardRuleDraft(campaignId))}>
                  {t('prep.rules.discard')}
                </Btn>
              </>
            ) : (
              <Panel title={t('prep.rules.coTitle')}>
                <p className="text-body text-chalk-soft">{t('prep.rules.coIdle')}</p>
              </Panel>
            )}
          </aside>
        </div>
      </div>
    </main>
  )
}

function PresetTab({ editor, busy, onStart }: { editor: RuleEditor; busy: boolean; onStart: () => void }) {
  const { t } = useTranslation()
  return (
    <Panel title={t('prep.rules.tab.preset')}>
      <p className="text-body text-chalk">{t('prep.rules.presetLine', { name: editor.name, version: editor.current })}</p>
      <p className="text-caption text-mute-soft">{t('prep.rules.presetHint')}</p>
      {!editor.draft && (
        <CardButton
          title={t('prep.rules.start')}
          subtitle={t('prep.rules.startSub', { version: editor.current })}
          disabled={busy}
          onClick={onStart}
        />
      )}
    </Panel>
  )
}

type Edit = (path: (string | number)[], value: unknown) => void

function StatsTab({ doc, edit }: { doc: RuleDocument; edit: Edit }) {
  const { t } = useTranslation()
  const abilities = listOf<Named>(doc, 'abilities')
  const stats = (doc.stats ?? {}) as Record<string, { name: string; abbr: string; formula: string }>
  return (
    <>
      <Panel title={t('prep.rules.abilities')}>
        {abilities.map((a, i) => (
          <div key={a.id} className="grid grid-cols-[60px_1fr_2fr] items-end gap-2">
            <b className="pb-2 font-mono text-caption">{a.id}</b>
            <TextField label={t('prep.rules.name')} value={a.name} onChange={(v) => edit(['abilities', i, 'name'], v)} />
            <TextField
              label={t('prep.rules.description')}
              value={a.description ?? ''}
              onChange={(v) => edit(['abilities', i, 'description'], v)}
            />
          </div>
        ))}
      </Panel>
      <Panel title={t('prep.rules.stats')}>
        {DERIVED.map((key) =>
          stats[key] ? (
            <div key={key} className="grid grid-cols-[1fr_80px_2fr] items-end gap-2">
              <TextField label={t('prep.rules.name')} value={stats[key].name} onChange={(v) => edit(['stats', key, 'name'], v)} />
              <TextField label={t('prep.rules.abbr')} value={stats[key].abbr} onChange={(v) => edit(['stats', key, 'abbr'], v)} />
              <TextField
                label={t('prep.rules.formula')}
                value={String(stats[key].formula)}
                onChange={(v) => edit(['stats', key, 'formula'], v)}
              />
            </div>
          ) : null,
        )}
        <TextField
          label={t('prep.rules.modifier')}
          value={String(doc.modifier ?? '')}
          onChange={(v) => edit(['modifier'], v)}
        />
      </Panel>
    </>
  )
}

function RollsTab({ doc, edit }: { doc: RuleDocument; edit: Edit }) {
  const { t } = useTranslation()
  const difficulties = listOf<Named & { value: number }>(doc, 'difficulties')
  const outcomes = (doc.outcomes ?? {}) as Record<string, { name?: string; description?: string }>
  const check = (doc.check ?? {}) as { dice?: string }
  return (
    <>
      <Panel title={t('prep.rules.roll')}>
        <p className="text-body text-chalk">{t('prep.rules.rollLine', { die: check.dice ?? '1d20' })}</p>
      </Panel>
      <Panel title={t('prep.rules.difficulties')}>
        {difficulties.map((d, i) => (
          <div key={d.id} className="grid grid-cols-[1fr_auto_2fr] items-end gap-2">
            <TextField label={t('prep.rules.name')} value={d.name} onChange={(v) => edit(['difficulties', i, 'name'], v)} />
            <NumberField label={t('prep.rules.value')} value={d.value} onChange={(v) => edit(['difficulties', i, 'value'], v)} />
            <TextField
              label={t('prep.rules.description')}
              value={d.description ?? ''}
              onChange={(v) => edit(['difficulties', i, 'description'], v)}
            />
          </div>
        ))}
      </Panel>
      <Panel title={t('prep.rules.outcomes')}>
        {BANDS.map((band) =>
          outcomes[band] ? (
            <TextField
              key={band}
              wide
              multiline
              label={outcomes[band].name ?? band}
              value={outcomes[band].description ?? ''}
              onChange={(v) => edit(['outcomes', band, 'description'], v)}
            />
          ) : null,
        )}
      </Panel>
    </>
  )
}

interface ActionDoc extends Named {
  kind: string
  level?: number
  tags?: Record<string, unknown>[]
}

/** The index of the first tag of `action` that holds `key`. */
function tagIndex(action: ActionDoc, key: string): number {
  return (action.tags ?? []).findIndex((tag) => key in tag)
}

function ActionsTab({ doc, edit }: { doc: RuleDocument; edit: Edit }) {
  const { t } = useTranslation()
  const kinds = listOf<Named & { cost: number }>(doc, 'action_kinds')
  const classes = listOf<Named & { actions: ActionDoc[] }>(doc, 'classes')
  return (
    <>
      <Panel title={t('prep.rules.kinds')}>
        <p className="text-caption text-mute-soft">{t('prep.rules.kindsHint')}</p>
        {kinds.map((k, i) => (
          <div key={k.id} className="grid grid-cols-[1fr_auto_2fr] items-end gap-2">
            <TextField label={t('prep.rules.name')} value={k.name} onChange={(v) => edit(['action_kinds', i, 'name'], v)} />
            <NumberField label={t('prep.rules.cost')} value={k.cost} onChange={(v) => edit(['action_kinds', i, 'cost'], v)} />
            <TextField
              label={t('prep.rules.description')}
              value={k.description ?? ''}
              onChange={(v) => edit(['action_kinds', i, 'description'], v)}
            />
          </div>
        ))}
      </Panel>
      {classes.map((c, ci) => (
        <Panel key={c.id} title={c.name}>
          {c.actions.map((a, ai) => {
            const dmg = tagIndex(a, DAMAGE)
            const cd = tagIndex(a, COOLDOWN)
            const damage = dmg >= 0 ? (a.tags![dmg].damage as { amount: unknown }) : null
            return (
              <div key={a.id} className="grid grid-cols-[1fr_2fr_auto_auto] items-end gap-2 border-t border-line pt-2">
                <TextField
                  label={t('prep.rules.card', { level: a.level ?? 1 })}
                  value={a.name}
                  onChange={(v) => edit(['classes', ci, 'actions', ai, 'name'], v)}
                />
                <TextField
                  label={t('prep.rules.description')}
                  value={a.description ?? ''}
                  onChange={(v) => edit(['classes', ci, 'actions', ai, 'description'], v)}
                />
                {damage ? (
                  <TextField
                    label={t('prep.rules.damage')}
                    value={String(damage.amount)}
                    onChange={(v) =>
                      edit(
                        ['classes', ci, 'actions', ai, 'tags', dmg, 'damage', 'amount'],
                        /^\d+$/.test(v) ? Number(v) : v,
                      )
                    }
                  />
                ) : (
                  <span />
                )}
                {cd >= 0 ? (
                  <NumberField
                    label={t('prep.rules.cooldown')}
                    value={Number(a.tags![cd].cooldown)}
                    onChange={(v) => edit(['classes', ci, 'actions', ai, 'tags', cd, 'cooldown'], v)}
                  />
                ) : (
                  <span />
                )}
              </div>
            )
          })}
        </Panel>
      ))}
    </>
  )
}

function HouseTab({ doc, edit }: { doc: RuleDocument; edit: Edit }) {
  const { t } = useTranslation()
  const rules = listOf<{ id: string; name: string; text: string }>(doc, 'house_rules')
  const [name, setName] = useState('')
  const [text, setText] = useState('')
  return (
    <>
      <Panel title={t('prep.rules.tab.house')}>
        <p className="text-caption text-mute-soft">{t('prep.rules.houseHint')}</p>
        {rules.length === 0 && <p className="text-body text-chalk-soft">{t('prep.rules.houseNone')}</p>}
        {rules.map((h, i) => (
          <div key={h.id} className="flex flex-col gap-2 border-t border-line pt-2">
            <TextField label={t('prep.rules.name')} value={h.name} onChange={(v) => edit(['house_rules', i, 'name'], v)} />
            <TextField
              label={t('prep.rules.houseText')}
              multiline
              value={h.text}
              onChange={(v) => edit(['house_rules', i, 'text'], v)}
            />
            <Btn
              className="self-end"
              onClick={() =>
                edit(
                  ['house_rules'],
                  rules.filter((_, j) => j !== i),
                )
              }
            >
              {t('prep.rules.remove')}
            </Btn>
          </div>
        ))}
      </Panel>
      <Panel title={t('prep.rules.houseNew')}>
        <TextField label={t('prep.rules.name')} value={name} onChange={setName} />
        <TextField label={t('prep.rules.houseText')} multiline value={text} onChange={setText} />
        <Btn
          main
          className="self-end"
          disabled={!name.trim() || !text.trim()}
          onClick={() => {
            const id = slugId(
              name,
              rules.map((r) => r.id),
            )
            edit(['house_rules'], [...rules, { id, name: name.trim(), text: text.trim() }])
            setName('')
            setText('')
          }}
        >
          {t('prep.rules.houseAdd')}
        </Btn>
      </Panel>
    </>
  )
}

function CreationTab({ doc, edit }: { doc: RuleDocument; edit: Edit }) {
  const { t } = useTranslation()
  const creation = (doc.creation ?? {}) as { free_action_slots?: number; note?: string }
  const classes = listOf<Named>(doc, 'classes')
  return (
    <>
      <Panel title={t('prep.rules.tab.creation')}>
        <p className="text-caption text-mute-soft">{t('prep.rules.creationHint')}</p>
        <NumberField
          label={t('prep.rules.freeSlots')}
          value={creation.free_action_slots ?? 0}
          onChange={(v) => edit(['creation', 'free_action_slots'], v)}
        />
        <TextField
          label={t('prep.rules.creationNote')}
          multiline
          wide
          value={creation.note ?? ''}
          onChange={(v) => edit(['creation', 'note'], v)}
        />
      </Panel>
      <Panel title={t('prep.rules.classes')}>
        {classes.map((c, i) => (
          <div key={c.id} className="grid grid-cols-[1fr_2fr] items-end gap-2">
            <TextField label={t('prep.rules.name')} value={c.name} onChange={(v) => edit(['classes', i, 'name'], v)} />
            <TextField
              label={t('prep.rules.description')}
              value={c.description ?? ''}
              onChange={(v) => edit(['classes', i, 'description'], v)}
            />
          </div>
        ))}
      </Panel>
    </>
  )
}

const pct = (x: number) => `${Math.round(x * 100)} %`

/** The right column: how much the draft touches, at a glance. */
function Touches({ report, onOpen }: { report: RuleReport; onOpen: () => void }) {
  const { t } = useTranslation()
  return (
    <Panel title={t('prep.rules.coTitle')} actions={<Btn onClick={onOpen}>{t('prep.rules.seeAll')}</Btn>}>
      <ul className="flex flex-col gap-1 text-body text-chalk-soft">
        <li>{t('prep.rules.touch.changes', { count: report.changes.length })}</li>
        <li className={report.story.length > 0 ? 'text-stat-atk' : undefined}>
          {t('prep.rules.touch.story', { count: report.story.length })}
        </li>
        <li>{t('prep.rules.touch.lint', { added: report.lintAdded.length, removed: report.lintRemoved.length })}</li>
        <li>{t('prep.rules.touch.fights', { count: report.fights.length })}</li>
      </ul>
    </Panel>
  )
}

function Issues({ issues }: { issues: { code: string; path: string; detail: string; message?: string }[] }) {
  return (
    <ul className="flex flex-col gap-1">
      {issues.map((i) => (
        <li key={`${i.code}${i.path}`} className="text-caption text-chalk-soft">
          <b className="text-chalk">{i.message ?? i.detail}</b> <span className="font-mono text-mute-soft">{i.path}</span>
        </li>
      ))}
    </ul>
  )
}

function TestTab({ report }: { report: RuleReport }) {
  const { t } = useTranslation()
  return (
    <>
      <Panel title={t('prep.rules.fights')}>
        <p className="text-caption text-mute-soft">{t('prep.rules.fightsHint')}</p>
        <table className="w-full text-left text-caption">
          <thead className="text-mute-soft">
            <tr>
              <th className="py-1">{t('prep.rules.fight')}</th>
              <th>{t('prep.rules.now')}</th>
              <th>{t('prep.rules.withDraft')}</th>
              <th>{t('prep.rules.rounds')}</th>
            </tr>
          </thead>
          <tbody>
            {report.fights.map((f) => (
              <tr key={`${f.source}${f.id}`} className="border-t border-line">
                <td className="py-1.5 text-chalk">
                  {f.name}
                  <span className="ml-1 text-mute-soft">{t(`prep.rules.source.${f.source}`)}</span>
                </td>
                <td>{f.current ? pct(f.current.partyWinRate) : '—'}</td>
                <td className={f.error ? 'text-stat-atk' : 'text-chalk'}>
                  {f.draft ? pct(f.draft.partyWinRate) : t('prep.rules.fightFailed')}
                </td>
                <td>{f.draft ? f.draft.rounds.toFixed(1) : '—'}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {report.fights
          .filter((f) => f.error)
          .map((f) => (
            <p key={f.id} className="text-caption text-stat-atk">
              {f.name} · {f.error}
            </p>
          ))}
      </Panel>
      <Panel title={t('prep.rules.changesTitle')}>
        {report.changes.length === 0 ? (
          <p className="text-body text-chalk-soft">{t('prep.rules.noChange')}</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {report.changes.map((c, i) => (
              <li key={i} className="text-caption text-chalk-soft"><RuleChangeLine item={c} /></li>
            ))}
          </ul>
        )}
      </Panel>
      <Panel title={t('prep.rules.storyTitle')}>
        {report.story.length === 0 ? (
          <p className="text-body text-chalk-soft">{t('prep.rules.storyNone')}</p>
        ) : (
          <Issues issues={report.story} />
        )}
      </Panel>
      <Panel title={t('prep.rules.lintTitle')}>
        <Section title={t('prep.rules.lintAdded', { count: report.lintAdded.length })}>
          <Issues issues={report.lintAdded} />
        </Section>
        <Section title={t('prep.rules.lintRemoved', { count: report.lintRemoved.length })}>
          <Issues issues={report.lintRemoved} />
        </Section>
        <Section title={t('prep.rules.lintAll', { count: report.lint.length })}>
          <Issues issues={report.lint} />
        </Section>
      </Panel>
    </>
  )
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <details className="flex flex-col gap-1">
      <summary className="cursor-pointer text-caption text-chalk">{title}</summary>
      {children}
    </details>
  )
}

function HistoryTab({ editor, campaignId }: { editor: RuleEditor; campaignId: string }) {
  const { t } = useTranslation()
  const versions = editor.versions
  const [from, setFrom] = useState(versions[versions.length - 1]?.version ?? editor.current)
  const [to, setTo] = useState(versions[0]?.version ?? editor.current)
  const [comparison, setComparison] = useState<Comparison | null>(null)
  const [failed, setFailed] = useState(false)
  const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'medium', timeStyle: 'short' })

  async function compare() {
    setFailed(false)
    try {
      setComparison(await compareRules(campaignId, from, to))
    } catch {
      setFailed(true)
    }
  }

  const label = (v: (typeof versions)[number]) =>
    v.preset
      ? t('prep.rules.versionPreset', { version: v.version })
      : v.lockedAt
        ? t('prep.rules.versionLocked', { version: v.version, date: dateFormat.format(new Date(v.lockedAt)) })
        : t('prep.rules.versionDraft', { version: v.version })

  return (
    <>
      <Panel title={t('prep.rules.tab.history')}>
        <ul className="flex flex-col gap-1.5">
          {versions.map((v) => (
            <li key={v.version} className="flex flex-col text-body">
              <span className={v.version === editor.current ? 'text-chalk' : 'text-chalk-soft'}>
                {label(v)}
                {v.version === editor.current && ` · ${t('prep.rules.played')}`}
              </span>
              {v.note && <span className="text-caption text-mute-soft">{v.note}</span>}
            </li>
          ))}
        </ul>
      </Panel>
      <Panel title={t('prep.rules.compare')}>
        <div className="flex flex-wrap items-end gap-2">
          {(
            [
              [t('prep.rules.from'), from, setFrom],
              [t('prep.rules.to'), to, setTo],
            ] as const
          ).map(([label, value, set]) => (
            <label key={label} className="flex flex-col gap-1 text-caption text-mute-soft">
              {label}
              <select className={field} value={value} onChange={(e) => set(Number(e.target.value))}>
                {versions.map((v) => (
                  <option key={v.version} value={v.version}>
                    {t('prep.rules.versionShort', { version: v.version })}
                  </option>
                ))}
              </select>
            </label>
          ))}
          <Btn main disabled={from === to} onClick={() => void compare()}>
            {t('prep.rules.compareGo')}
          </Btn>
        </div>
        {failed && (
          <p role="alert" className="text-caption text-stat-atk">
            {t('prep.rules.errors.action')}
          </p>
        )}
        {comparison && (
          <>
            <h3 className="type-label text-chalk">{t('prep.rules.changesTitle')}</h3>
            {comparison.changes.length === 0 ? (
              <p className="text-body text-chalk-soft">{t('prep.rules.noChange')}</p>
            ) : (
              <ul className="flex flex-col gap-1">
                {comparison.changes.map((c, i) => (
                  <li key={i} className="text-caption text-chalk-soft"><RuleChangeLine item={c} /></li>
                ))}
              </ul>
            )}
            <h3 className="type-label text-chalk">{t('prep.rules.lines')}</h3>
            <pre className="max-h-[50vh] overflow-auto rounded-button border border-line bg-table p-2 text-[12px]">
              {comparison.lines.map((l, i) => (
                <div
                  key={i}
                  className={cn(
                    l.kind === 'added' && 'text-stat-move',
                    l.kind === 'removed' && 'text-stat-atk line-through',
                    l.skipped !== undefined && 'text-mute-soft',
                  )}
                >
                  {l.skipped !== undefined
                    ? t('prep.rules.skipped', { count: l.skipped })
                    : `${l.kind === 'added' ? '+' : l.kind === 'removed' ? '−' : ' '} ${l.text}`}
                </div>
              ))}
            </pre>
          </>
        )}
      </Panel>
    </>
  )
}
