import { useEffect, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useParams } from 'react-router'
import { CardButton } from '@/components/game/CardButton'
import { GameCard } from '@/components/game/GameCard'
import { RollDetail } from '@/components/game/RollDetail'
import { StatusBanner } from '@/components/game/StatusBanner'
import { ApiError } from '@/lib/api'
import { playPath } from '@/lib/play'
import {
  fetchRules,
  markRulesSeen,
  type ModifierSource,
  type OutcomeBand,
  type RuleCardView,
  type RulesView,
} from '@/lib/rules'
import { cn } from '@/lib/utils'
import { signed } from '../creator/RuleSteps'
import { RuleChangeLine } from '@/components/game/RuleChangeLine'

type PageState =
  | { kind: 'loading' }
  | { kind: 'not-joined' }
  | { kind: 'error' }
  | { kind: 'ready'; rules: RulesView }

/** Glyphs, not words. */

/**
 * `/partie/:campaignId/regles` — the campaign's rules on one page, phone
 * first. Everything is the rule system as the server projects it
 * (`GET /api/play/…/rules`): nothing here is written by hand, so the
 * page cannot drift from the rules the server applies. With a sheet,
 * the player's own numbers: what they need on the die, each card's
 * cost, cooldown and attack roll, and worked rolls made by the engine.
 * What changed since the version last read comes first, until the
 * player says they read it.
 */
export function RulesPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    let live = true
    fetchRules(campaignId).then(
      (rules) => {
        if (live) setState({ kind: 'ready', rules })
      },
      (err: unknown) => {
        if (!live) return
        setState({ kind: err instanceof ApiError && err.status === 401 ? 'not-joined' : 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [campaignId])

  if (state.kind !== 'ready') {
    return (
      <main className="surface-table mx-auto flex min-h-dvh max-w-md flex-col p-4 text-chalk">
        {state.kind === 'loading' && <p role="status">{t('play.loading')}</p>}
        {state.kind === 'not-joined' && <p role="alert">{t('play.notJoined')}</p>}
        {state.kind === 'error' && <p role="alert">{t('rules.error')}</p>}
      </main>
    )
  }

  const { rules } = state
  const acknowledge = () => {
    setSaving(true)
    markRulesSeen(campaignId).then(
      (next) => {
        setState({ kind: 'ready', rules: next })
        setSaving(false)
      },
      () => setSaving(false),
    )
  }

  return (
    <main className="surface-table mx-auto flex min-h-dvh max-w-md flex-col text-chalk">
      <StatusBanner tone={rules.changes ? 'you' : 'listen'} className="mx-3 mt-4">
        {t(rules.changes ? 'rules.banner.changed' : 'rules.banner.read')}
      </StatusBanner>
      <div className="flex flex-col gap-5 p-4">
        <header className="flex flex-col gap-1">
          <span className="type-label">{t('rules.version', { version: rules.version })}</span>
          <h1 className="type-title text-heading">{rules.name}</h1>
        </header>
        {rules.changes && <Changes rules={rules} saving={saving} onRead={acknowledge} />}
        <Roll rules={rules} />
        {rules.mine && <MyRolls rules={rules} />}
        <Attack rules={rules} />
        {rules.mine && <MyCards rules={rules} />}
        <Turn rules={rules} />
        <Conditions rules={rules} />
        <Between rules={rules} />
        <CardButton
          variant="dark"
          title={t('rules.back')}
          onClick={() => navigate(playPath(campaignId))}
        />
      </div>
    </main>
  )
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-2.5">
      <h2 className="type-title text-[20px]">{title}</h2>
      {children}
    </section>
  )
}

function Row({ label, children }: { label: ReactNode; children: ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-3 border-b border-line py-1.5 text-body">
      <span className="text-chalk-soft">{label}</span>
      <span className="text-right font-bold">{children}</span>
    </div>
  )
}

function bandNamer(rules: RulesView) {
  return (band: OutcomeBand) => rules.outcomes.find((o) => o.band === band)?.name ?? band
}

function useSourceName(rules: RulesView) {
  const { t } = useTranslation()
  return (s: ModifierSource): string => {
    switch (s.from) {
      case 'ability':
        return rules.abilities.find((a) => a.id === s.id)?.name ?? s.id
      case 'precision':
        return t('rules.source.precision', {
          card: rules.mine?.cards.find((c) => c.id === s.id)?.name ?? s.id,
        })
      case 'cover':
        return t('rules.source.cover')
      case 'long_range':
        return t('rules.source.longRange')
      default:
        return s.id
    }
  }
}

/** What changed since the version the player last read. */
function Changes({ rules, saving, onRead }: { rules: RulesView; saving: boolean; onRead: () => void }) {
  const { t } = useTranslation()
  const c = rules.changes!
  return (
    <section className="flex flex-col gap-2.5 rounded-xl bg-ivory p-3.5 text-ink shadow-ivory-flat">
      <h2 className="type-title text-[20px]">
        {t('rules.changes.title', { from: c.fromVersion, to: c.toVersion })}
      </h2>
      <p className="text-body">{t('rules.changes.when')}</p>
      {c.replaced ? (
        <p className="text-body font-bold">{t('rules.changes.replaced')}</p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {c.items.map((item, i) => (
            <li key={i} className="text-body">
              <RuleChangeLine item={item} />
            </li>
          ))}
        </ul>
      )}
      <CardButton
        variant="dark"
        title={t('rules.changes.read')}
        subtitle={t('rules.changes.readHint')}
        disabled={saving}
        onClick={onRead}
      />
    </section>
  )
}

/** The roll: the die, the modifier, and the four outcomes. */
function Roll({ rules }: { rules: RulesView }) {
  const { t } = useTranslation()
  return (
    <Section title={t('rules.roll.title')}>
      <p className="text-body text-chalk-soft">
        {t('rules.roll.how', { die: rules.check.die })}
      </p>
      <Row label={t('rules.roll.modifier')}>{rules.check.modifier}</Row>
      {rules.check.advantage && <p className="text-caption text-mute">{t('rules.roll.advantage')}</p>}
      <div className="grid grid-cols-2 gap-2">
        {rules.outcomes.map((o) => (
          <div
            key={o.band}
            className={cn(
              'flex flex-col gap-1 rounded-xl p-2.5 text-caption',
              o.band.includes('success') ? 'bg-ivory text-ink shadow-ivory-flat' : 'border border-line bg-surface',
            )}
          >
            <b className="text-body">{o.name}</b>
            <span className="font-bold">
              {o.natural.length > 0
                ? t('rules.roll.natural', { faces: o.natural.join(', ') })
                : t(o.band === 'success' ? 'rules.roll.reach' : 'rules.roll.miss')}
            </span>
            <span>{o.description}</span>
            {o.xp > 0 && <span className="font-bold">{t('rules.roll.xp', { count: o.xp })}</span>}
          </div>
        ))}
      </div>
      <h3 className="type-label mt-1">{t('rules.roll.difficulties')}</h3>
      {rules.difficulties.map((d) => (
        <Row key={d.id} label={<><b className="text-chalk">{d.name}</b> {d.description}</>}>
          {d.value}
        </Row>
      ))}
      {rules.groupCheck && (
        <p className="text-caption text-chalk-soft">{t(`rules.group.${rules.groupCheck}`)}</p>
      )}
    </Section>
  )
}

/** What the player needs on the die, ability by ability, and worked rolls. */
function MyRolls({ rules }: { rules: RulesView }) {
  const { t } = useTranslation()
  const sourceName = useSourceName(rules)
  const mine = rules.mine!
  const need = (face: number | null) => (face === null ? t('rules.mine.never') : t('rules.mine.from', { face }))
  return (
    <Section title={t('rules.mine.title')}>
      <p className="text-body text-chalk-soft">{t('rules.mine.how', { die: rules.check.die })}</p>
      <div className="overflow-x-auto">
        <table className="w-full text-caption">
          <thead>
            <tr className="text-mute">
              <th className="py-1 text-left font-semibold">{t('rules.mine.ability')}</th>
              {rules.difficulties.map((d) => (
                <th key={d.id} className="px-1 py-1 text-center font-semibold">
                  {d.name}
                  <span className="block">{d.value}</span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rules.abilities.map((a) => (
              <tr key={a.id} className="border-t border-line">
                <th scope="row" className="py-1.5 text-left font-semibold">
                  {a.name}
                  {a.modifier !== null && <span className="ml-1 text-chalk-soft">{signed(a.modifier)}</span>}
                </th>
                {a.needs.map((n) => (
                  <td key={n.difficulty} className="px-1 text-center font-bold tabular-nums">
                    {need(n.fromFace)}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <h3 className="type-label mt-1">
        {t('rules.mine.examples', { ability: mine.exampleAbility, difficulty: mine.exampleDifficulty })}
      </h3>
      <div className="flex flex-col gap-2">
        {mine.examples.map((roll) => (
          <RollDetail
            key={roll.natural}
            roll={roll}
            bandName={bandNamer(rules)}
            sourceName={sourceName}
            targetName={mine.exampleDifficulty}
          />
        ))}
      </div>
    </Section>
  )
}

/** How an attack is rolled and against what. */
function Attack({ rules }: { rules: RulesView }) {
  const { t } = useTranslation()
  const crit = rules.outcomes.find((o) => o.band === 'critical_success')
  const fumble = rules.outcomes.find((o) => o.band === 'critical_failure')
  const ac = rules.attack.armorClass
  return (
    <Section title={t('rules.attack.title')}>
      <p className="text-body text-chalk-soft">
        {t('rules.attack.how', { die: rules.check.die, ac: ac.name })}
      </p>
      <Row label={t('rules.attack.ability')}>{t(`rules.codes.${rules.attack.ability}`)}</Row>
      <Row label={t('rules.attack.precision')}>
        {t(rules.attack.precisionApplies ? 'rules.codes.added_to_attack_roll' : 'rules.codes.not_applied')}
      </Row>
      <Row label={t('rules.attack.armorClass', { name: ac.name, abbr: ac.abbr })}>{ac.formula}</Row>
      <Row label={t('rules.attack.hitPoints', { name: rules.hitPoints.name, abbr: rules.hitPoints.abbr })}>
        {rules.hitPoints.formula}
      </Row>
      {fumble && fumble.natural.length > 0 && (
        <p className="text-caption text-chalk-soft">
          {t('rules.attack.fumble', { faces: fumble.natural.join(', ') })}
        </p>
      )}
      {crit && crit.natural.length > 0 && (
        <p className="text-caption text-chalk-soft">
          {crit.damageMultiplier
            ? t('rules.attack.critMultiplier', { faces: crit.natural.join(', '), times: crit.damageMultiplier })
            : t('rules.attack.crit', { faces: crit.natural.join(', ') })}
        </p>
      )}
    </Section>
  )
}

/** Each card: cost in actions, cooldown, and what its attack roll adds. */
function MyCards({ rules }: { rules: RulesView }) {
  const { t } = useTranslation()
  const mine = rules.mine!
  return (
    <Section title={t('rules.cards.title', { name: mine.className })}>
      <p className="text-caption text-chalk-soft">{t(`rules.codes.${rules.cooldown}`)}</p>
      {mine.cards.map((card) => (
        <CardRow key={card.id} card={card} rules={rules} />
      ))}
    </Section>
  )
}

function CardRow({ card, rules }: { card: RuleCardView; rules: RulesView }) {
  const { t } = useTranslation()
  const sourceName = useSourceName(rules)
  const gem =
    card.attackBonus !== null
      ? { stat: 'atk' as const, value: signed(card.attackBonus) }
      : card.heal
        ? { stat: 'hp' as const, value: card.heal }
        : undefined
  return (
    <div className="flex gap-3 border-b border-line pb-3">
      <GameCard
        kind="action"
        title={card.name}
        typeLabel={card.kind}
        text={card.damage ? t('creator.class.damage', { amount: card.damage }) : card.description}
        value={gem?.value}
        stat={gem?.stat}
        icon={card.heal ? 'spell' : 'sword'}
        width={96}
        deal={false}
        state={card.level > 1 ? 'greyed' : 'normal'}
      />
      <div className="flex min-w-0 flex-1 flex-col gap-1 text-caption">
        <b className="text-body">{card.name}</b>
        <span className="font-bold">{t('rules.cards.cost', { count: card.cost })}</span>
        <span className="font-bold">
          {card.cooldown > 0 ? t('rules.cards.cooldown', { count: card.cooldown }) : t('rules.cards.noCooldown')}
        </span>
        {card.attackModifiers && (
          <span>
            {t('rules.cards.attack', {
              die: rules.check.die,
              mods: card.attackModifiers
                .map((m) => `${signed(m.value)} ${sourceName(m.source)}`)
                .join(' '),
              ac: rules.attack.armorClass.abbr,
            })}
          </span>
        )}
        {card.heal && <span>{t('rules.cards.heal', { amount: card.heal })}</span>}
        <span className="text-chalk-soft">{t('rules.cards.range', { count: card.range })}</span>
        {card.level > 1 && <span className="text-mute">{t('rules.cards.level', { level: card.level })}</span>}
      </div>
    </div>
  )
}

/** The turn: actions per turn, what each kind costs, cooldowns, durations, the grid. */
function Turn({ rules }: { rules: RulesView }) {
  const { t } = useTranslation()
  const c = rules.combat
  return (
    <Section title={t('rules.turn.title')}>
      {rules.turns.map((turn) => (
        <div key={turn.name} className="flex flex-col gap-1">
          <Row label={turn.name}>{t('rules.turn.actions', { count: turn.actionsPerTurn })}</Row>
          {turn.limits.map((l) => (
            <p key={l.kind} className="text-caption text-chalk-soft">
              {t('rules.turn.limit', { kind: l.kind, count: l.maxPerTurn })}
            </p>
          ))}
        </div>
      ))}
      <h3 className="type-label mt-1">{t('rules.turn.kinds')}</h3>
      {rules.actionKinds.map((k) => (
        <Row key={k.name} label={<><b className="text-chalk">{k.name}</b> {k.description}</>}>
          {t('rules.cards.cost', { count: k.cost })}
        </Row>
      ))}
      <p className="text-caption text-chalk-soft">{t(`rules.codes.${rules.cooldown}`)}</p>
      <p className="text-caption text-chalk-soft">
        {t(rules.applicationTurnCounts ? 'rules.turn.durationCounts' : 'rules.turn.durationNext')}
      </p>
      <h3 className="type-label mt-1">{t('rules.grid.title')}</h3>
      <Row label={t('rules.grid.move')}>
        {c.moveKind ? t('rules.grid.moveKind', { kind: c.moveKind.name, count: c.moveKind.cost }) : t('rules.grid.moveFree')}
      </Row>
      {c.flee && (
        <Row label={t('rules.grid.flee')}>
          {c.flee.ability
            ? t('rules.grid.fleeAbility', { count: c.flee.kind.cost, ability: c.flee.ability })
            : t('rules.cards.cost', { count: c.flee.kind.cost })}
        </Row>
      )}
      <Row label={t('rules.grid.coverHalf')}>{signed(c.coverHalf)}</Row>
      <Row label={t('rules.grid.coverThreeQuarters')}>{signed(c.coverThreeQuarters)}</Row>
      <Row label={t('rules.grid.longRange')}>
        {c.longRangeModifier !== null ? signed(c.longRangeModifier) : t('rules.codes.disadvantage')}
      </Row>
    </Section>
  )
}

function Conditions({ rules }: { rules: RulesView }) {
  const { t } = useTranslation()
  return (
    <Section title={t('rules.conditions.title')}>
      {rules.conditions.map((c) => (
        <div key={c.name} className="flex items-start gap-2.5 border-b border-line py-1.5 text-body">
          <span
            className={cn(
              'mt-0.5 flex-none rounded-[4px] px-1.5 text-label font-bold uppercase',
              c.kind === 'boon' ? 'bg-ivory text-ink' : 'border border-chalk text-chalk',
            )}
          >
            {t(`rules.conditions.${c.kind}`)}
          </span>
          <span>
            <b>{c.name}</b> {c.description}
          </span>
        </div>
      ))}
    </Section>
  )
}

/** Zero hit points and progression. */
function Between({ rules }: { rules: RulesView }) {
  const { t } = useTranslation()
  const z = rules.zeroHp
  const p = rules.progression
  return (
    <>
      <Section title={t('rules.zeroHp.title', { abbr: rules.hitPoints.abbr })}>
        <p className="text-body text-chalk-soft">
          {z.rule === 'knockedOut'
            ? t('rules.zeroHp.knockedOut', { condition: z.condition, count: z.outAfterTurns, out: z.outCondition })
            : t('rules.zeroHp.deathSaves', { difficulty: z.difficulty, successes: z.successes, failures: z.failures })}
        </p>
        {z.rule === 'deathSaves' && (
          <p className="text-body text-chalk-soft">
            {t('rules.zeroHp.criticals', { count: z.failuresOnCriticalFailure })}
            {z.criticalSuccessRevives && ` ${t('rules.zeroHp.revives')}`}{' '}
            {t('rules.zeroHp.hit', { count: z.failuresOnHit })}
            {z.stabilize &&
              ` ${t('rules.zeroHp.stabilize', { kind: z.stabilize.kind, ability: z.stabilize.ability, difficulty: z.stabilize.difficulty })}`}
          </p>
        )}
        <p className="text-caption text-mute">{t('rules.zeroHp.gm')}</p>
      </Section>
      <Section title={t('rules.progression.title')}>
        <p className="text-body text-chalk-soft">
          {t('rules.progression.upgrade', { xp: p.upgradeEveryXp, count: p.upgradePoints })}
        </p>
        {p.hitPointsPerLevel && (
          <p className="text-body text-chalk-soft">
            {t(p.hitPointsPerLevel.ability ? 'rules.progression.hitPoints' : 'rules.progression.hitPointsPlain', {
              dice: p.hitPointsPerLevel.dice,
              average: p.hitPointsPerLevel.average,
              ability: p.hitPointsPerLevel.ability,
            })}
          </p>
        )}
        <div className="flex flex-wrap gap-1.5">
          {p.levels.map((l) => (
            <span key={l.level} className="rounded-button border border-line bg-surface px-2 py-1 text-caption">
              {t('rules.progression.level', { level: l.level, xp: l.xp })}
            </span>
          ))}
        </div>
      </Section>
      {rules.houseRules.length > 0 && (
        <Section title={t('rules.houseRules.title')}>
          <p className="text-caption text-mute-soft">{t('rules.houseRules.hint')}</p>
          {rules.houseRules.map((h) => (
            <div key={h.name} className="flex flex-col gap-1">
              <b className="type-label text-chalk">{h.name}</b>
              <p className="text-body text-chalk-soft">{h.text}</p>
            </div>
          ))}
        </Section>
      )}
    </>
  )
}
