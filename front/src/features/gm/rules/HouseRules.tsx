import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { Btn, Panel } from '@/features/gm/live/ui'
import { ApiError } from '@/lib/api'
import {
  formaliseHouseRule,
  listOf,
  slugId,
  tryHouseRule,
  type CaseResult,
  type FormalRule,
  type HouseRuleDoc,
  type Proposal,
  type RuleDocument,
  type RuleReport,
} from '@/lib/ruleEditor'
import { cn } from '@/lib/utils'
import { TextField } from '../fields'

/** The board's marks: the rule fired, or it stayed silent. */
const FIRED = '✓'
const SILENT = '—'

type Edit = (path: (string | number)[], value: unknown) => void

interface Named {
  id: string
  name: string
}

/** Looks up the name of an id in one of the document's lists. */
function namer(doc: RuleDocument) {
  return (list: string, id: string) => listOf<Named>(doc, list).find((x) => x.id === id)?.name ?? id
}

type Ask =
  | { kind: 'idle' }
  | { kind: 'reading' }
  | { kind: 'proposal'; proposal: Proposal }
  | { kind: 'error'; code: string }

/**
 * The « Règles maison » tab (engine/formalise-house-rules, planche
 * « Règles », moment 5): the GM writes a rule in French; the co-GM
 * proposes its formal form — when, effect, exception, what players see
 * — with cases the server replays at once; the GM adds it to the draft.
 * Nothing is stored before that, and the draft's save replays its cases.
 */
export function HouseTab({
  doc,
  edit,
  campaignId,
  report,
}: {
  doc: RuleDocument
  edit: Edit
  campaignId: string
  report: RuleReport
}) {
  const { t } = useTranslation()
  const rules = listOf<HouseRuleDoc>(doc, 'house_rules')
  const [name, setName] = useState('')
  const [text, setText] = useState('')
  return (
    <>
      <Panel title={t('prep.rules.tab.house')}>
        <p className="text-caption text-mute-soft">{t('prep.rules.houseHint')}</p>
        {rules.length === 0 && <p className="text-body text-chalk-soft">{t('prep.rules.houseNone')}</p>}
        {rules.map((h, i) => (
          <HouseRuleRow
            key={h.id}
            rule={h}
            doc={doc}
            campaignId={campaignId}
            saved={report.houseRules.find((c) => c.id === h.id)?.cases ?? null}
            onEdit={(field, value) => edit(['house_rules', i, field], value)}
            onFormal={(formal) => {
              const next = { ...h }
              if (formal) next.formal = formal
              else delete next.formal
              edit(['house_rules', i], next)
            }}
            onRemove={() =>
              edit(
                ['house_rules'],
                rules.filter((_, j) => j !== i),
              )
            }
          />
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

function HouseRuleRow({
  rule,
  doc,
  campaignId,
  saved,
  onEdit,
  onFormal,
  onRemove,
}: {
  rule: HouseRuleDoc
  doc: RuleDocument
  campaignId: string
  /** The cases the last save replayed, when the rule was saved formalised. */
  saved: CaseResult[] | null
  onEdit: (field: 'name' | 'text', value: string) => void
  onFormal: (formal: FormalRule | null) => void
  onRemove: () => void
}) {
  const { t } = useTranslation()
  const [ask, setAsk] = useState<Ask>({ kind: 'idle' })

  async function run(call: () => Promise<Proposal>) {
    setAsk({ kind: 'reading' })
    try {
      setAsk({ kind: 'proposal', proposal: await call() })
    } catch (err) {
      setAsk({ kind: 'error', code: err instanceof ApiError ? err.code : 'action' })
    }
  }

  const formalise = () =>
    void run(() => formaliseHouseRule(campaignId, { id: rule.id, name: rule.name, text: rule.text }))

  return (
    <div className="flex flex-col gap-2 border-t border-line pt-2">
      <TextField label={t('prep.rules.name')} value={rule.name} onChange={(v) => onEdit('name', v)} />
      <TextField label={t('prep.rules.houseText')} multiline value={rule.text} onChange={(v) => onEdit('text', v)} />
      {rule.formal ? (
        <>
          <p className="text-caption text-stat-move">{t('prep.rules.formal.judged')}</p>
          <FormalSummary formal={rule.formal} doc={doc} />
          {saved && <Cases cases={saved} />}
        </>
      ) : (
        <p className="text-caption text-mute-soft">{t('prep.rules.formal.none')}</p>
      )}

      {ask.kind === 'reading' && (
        <p role="status" className="text-caption text-chalk-soft">
          {t('prep.rules.formal.reading')}
        </p>
      )}
      {ask.kind === 'error' && (
        <p role="alert" className="text-caption text-stat-atk">
          {t(`prep.rules.formal.errors.${ask.code}`, {
            defaultValue: t(`prep.rules.errors.${ask.code}`, { defaultValue: t('prep.rules.formal.errors.action') }),
          })}
        </p>
      )}
      {ask.kind === 'proposal' && (
        <ProposalView
          proposal={ask.proposal}
          doc={doc}
          onPlayers={(players) =>
            setAsk({
              kind: 'proposal',
              proposal: { ...ask.proposal, formal: { ...ask.proposal.formal, players } },
            })
          }
          onRetry={() => void run(() => tryHouseRule(campaignId, { ...rule, formal: ask.proposal.formal }))}
          onAdd={() => {
            onFormal(ask.proposal.formal)
            setAsk({ kind: 'idle' })
          }}
          onDismiss={() => setAsk({ kind: 'idle' })}
        />
      )}

      <div className="flex flex-wrap justify-end gap-2">
        {rule.formal ? (
          <Btn onClick={() => onFormal(null)}>{t('prep.rules.formal.unformalise')}</Btn>
        ) : (
          ask.kind !== 'proposal' && (
            <CardButton
              title={t('prep.rules.formal.ask')}
              subtitle={t('prep.rules.formal.askSub')}
              disabled={ask.kind === 'reading' || !rule.text.trim()}
              onClick={formalise}
            />
          )
        )}
        <Btn onClick={onRemove}>{t('prep.rules.remove')}</Btn>
      </div>
    </div>
  )
}

function ProposalView({
  proposal,
  doc,
  onPlayers,
  onRetry,
  onAdd,
  onDismiss,
}: {
  proposal: Proposal
  doc: RuleDocument
  onPlayers: (players: 'rule' | 'effect') => void
  onRetry: () => void
  onAdd: () => void
  onDismiss: () => void
}) {
  const { t } = useTranslation()
  const shown = (proposal.formal.players ?? 'rule') === 'rule'
  return (
    <section
      className="flex flex-col gap-2 rounded-button border border-line-strong p-2.5"
      aria-label={t('prep.rules.formal.proposal')}
    >
      <h3 className="type-label text-chalk">{t('prep.rules.formal.proposal')}</h3>
      {proposal.remark && <p className="text-caption text-chalk-soft italic">{proposal.remark}</p>}
      <FormalSummary formal={proposal.formal} doc={doc} />
      <label className="flex items-center gap-2 text-caption text-chalk">
        <input type="checkbox" checked={shown} onChange={(e) => onPlayers(e.target.checked ? 'rule' : 'effect')} />
        {t('prep.rules.formal.showRule')}
      </label>
      {proposal.dropped.length > 0 && (
        <p className="text-caption text-stat-init">
          {t('prep.rules.formal.dropped', {
            items: proposal.dropped
              .map((d) => t(`prep.rules.formal.droppedKind.${d.kind}`, { id: d.id }))
              .join(', '),
          })}
        </p>
      )}
      {proposal.problems.length > 0 && (
        <div role="alert" className="flex flex-col gap-1 text-caption text-stat-atk">
          <span>{t('prep.rules.formal.problems')}</span>
          <ul>
            {proposal.problems.map((p) => (
              <li key={`${p.code}${p.path}`}>
                <span className="font-mono">{p.path}</span> — {p.detail}
              </li>
            ))}
          </ul>
        </div>
      )}
      {proposal.problems.length === 0 && <Cases cases={proposal.cases} />}
      <div className="flex flex-wrap justify-end gap-2">
        <Btn onClick={onDismiss}>{t('prep.rules.formal.dismiss')}</Btn>
        <Btn onClick={onRetry}>{t('prep.rules.formal.retry')}</Btn>
        <CardButton
          title={t('prep.rules.formal.add')}
          subtitle={t('prep.rules.formal.addSub')}
          disabled={proposal.problems.length > 0}
          onClick={onAdd}
        />
      </div>
    </section>
  )
}

/** The formal rule as the board reads it: when, effect, exception, players. */
function FormalSummary({ formal, doc }: { formal: FormalRule; doc: RuleDocument }) {
  const { t } = useTranslation()
  const name = namer(doc)
  const trigger = (
    formal.critical === undefined
      ? formal.when
      : `${formal.when}${formal.critical ? 'Critical' : 'NotCritical'}`
  ) as 'hit' | 'hitCritical' | 'hitNotCritical' | 'miss' | 'missCritical' | 'missNotCritical'
  const who = (w: FormalRule['actor']) =>
    [
      w?.side && t(`prep.rules.formal.side.${w.side}`),
      w?.traits?.length && t('prep.rules.formal.traits', { traits: w.traits.map((x) => name('traits', x)).join(', ') }),
    ]
      .filter(Boolean)
      .join(' ')
  const when = [
    t(`prep.rules.formal.trigger.${trigger}`),
    formal.damage_type && t('prep.rules.formal.damageType', { type: name('damage_types', formal.damage_type) }),
    who(formal.actor) && t('prep.rules.formal.actorIs', { who: who(formal.actor) }),
    who(formal.target) && t('prep.rules.formal.targetIs', { who: who(formal.target) }),
  ]
    .filter(Boolean)
    .join(' · ')
  const origin = (id: string) => (listOf<Named>(doc, 'classes').some((c) => c.id === id) ? name('classes', id) : name('adversaries', id))
  const exceptions = [formal.actor, formal.target].flatMap((w) => [
    ...(w?.except_traits?.length
      ? [t('prep.rules.formal.exceptTraits', { traits: w.except_traits.map((x) => name('traits', x)).join(', ') })]
      : []),
    ...(w?.except?.length ? [t('prep.rules.formal.exceptIds', { ids: w.except.map(origin).join(', ') })] : []),
  ])
  const effects = formal.effects.map((e) => {
    if ('apply' in e) {
      const a = e.apply
      const save = a.save
        ? ` ${t('prep.rules.formal.save', {
            ability: name('abilities', a.save.ability),
            difficulty: a.save.difficulty ? name('difficulties', a.save.difficulty) : '',
          })}`
        : ''
      return (
        t('prep.rules.formal.apply', {
          condition: a.condition ? name('conditions', a.condition) : '',
          count: a.turns,
          to: t(`prep.rules.formal.to.${a.to}`),
        }) + save
      )
    }
    const [kind, v] = 'damage' in e ? (['damage', e.damage] as const) : (['heal', e.heal] as const)
    return t(`prep.rules.formal.${kind}`, { amount: String(v.amount), to: t(`prep.rules.formal.to.${v.to}`) })
  })
  const rows: [string, string][] = [
    [t('prep.rules.formal.when'), when],
    [t('prep.rules.formal.effect'), effects.join(' ; ')],
    [t('prep.rules.formal.except'), exceptions.length ? exceptions.join(' ; ') : t('prep.rules.formal.noException')],
    [
      t('prep.rules.formal.players'),
      t((formal.players ?? 'rule') === 'rule' ? 'prep.rules.formal.seeRule' : 'prep.rules.formal.seeEffect'),
    ],
  ]
  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 rounded-button border border-line bg-table p-2 text-caption">
      {rows.map(([label, value]) => (
        <div key={label} className="contents">
          <dt className="type-label text-mute-soft">{label}</dt>
          <dd className="text-chalk">{value}</dd>
        </div>
      ))}
    </dl>
  )
}

/** The cases, as the server replayed them: ✓ the rule fired, — it did not. */
function Cases({ cases }: { cases: CaseResult[] }) {
  const { t } = useTranslation()
  const who = (id?: string) => t(id === 'actor' ? 'prep.rules.formal.whoActor' : 'prep.rules.formal.whoTarget')
  return (
    <div className="flex flex-col gap-1">
      <span className="type-label text-mute-soft">{t('prep.rules.formal.cases')}</span>
      {cases.length === 0 && <p className="text-caption text-chalk-soft">{t('prep.rules.formal.casesNone')}</p>}
      <ul className="flex flex-col gap-1">
        {cases.map((c) => {
          const did = c.effects
            .map((e) =>
              e.event === 'condition_applied'
                ? t('fight.log.condition', { target: who(e.target), name: e.name })
                : e.event === 'damaged'
                  ? t('fight.log.damaged', {
                      target: who(e.target),
                      amount: (e.hp_before ?? 0) - (e.hp_after ?? 0),
                    })
                  : e.event === 'healed'
                    ? t('fight.log.healed', { target: who(e.target), amount: e.amount })
                    : null,
            )
            .filter(Boolean)
            .join(', ')
          return (
            <li
              key={c.name}
              className={cn(
                'flex flex-wrap items-baseline gap-x-2 text-caption',
                c.passed ? 'text-chalk' : 'text-stat-atk',
              )}
            >
              <b aria-hidden className="w-4 text-center">
                {c.fired ? FIRED : SILENT}
              </b>
              <span>{c.name}</span>
              {c.natural !== null && (
                <span className="text-mute-soft">{t('prep.rules.formal.natural', { face: c.natural })}</span>
              )}
              {did && <span className="text-chalk-soft">{did}</span>}
              <span className="text-mute-soft">
                {t(c.expect === 'applies' ? 'prep.rules.formal.expectApplies' : 'prep.rules.formal.expectNothing')}
                {' · '}
                {c.error
                  ? t('prep.rules.formal.caseError', { detail: c.error })
                  : t(c.passed ? 'prep.rules.formal.casePassed' : 'prep.rules.formal.caseFailed')}
              </span>
            </li>
          )
        })}
      </ul>
    </div>
  )
}
