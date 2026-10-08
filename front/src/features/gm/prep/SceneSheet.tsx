import { useEffect, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { MusicPlayer } from '@/features/evening/MusicPlayer'
import { Btn, field } from '@/features/gm/live/ui'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { MapCanvas } from '@/features/map/MapCanvas'
import { useImage } from '@/features/map/useImage'
import { useTileset } from '@/features/map/useTileset'
import { ApiError } from '@/lib/api'
import { youtubeId } from '@/lib/evening'
import {
  deleteMap,
  fetchMaps,
  generateMap,
  gmBackdropUrl,
  validateMap,
  type CampaignMap,
  type MapListing,
} from '@/lib/maps'
import { askImage, decideImage, fetchGmMedia, gmImageUrl, type GmMediaList, type MediaKind } from '@/lib/media'
import {
  freshId,
  nameOf,
  type Ambience,
  type Edit,
  type Encounter,
  type Entity,
  type Loot,
  type MusicMood,
  type MusicTrack,
  type Opponents,
  type PlannedCheck,
  type Story,
  type StoryNode,
} from '@/lib/prep'
import { cn } from '@/lib/utils'
import { TextField } from '../fields'
import { EntityFields, useEntityFields, type FieldDef } from './EntityForm'
import { SubjectCard } from './SubjectCard'

const SHEET_TABS = ['text', 'combat', 'visuals', 'sound'] as const
type SheetTab = (typeof SHEET_TABS)[number]

const TEXT_FIELDS: FieldDef[] = [
  { path: 'title' },
  { path: 'summary', multiline: true },
  { path: 'read_aloud', multiline: true },
  { path: 'hook', multiline: true },
  { path: 'flow', multiline: true },
  { path: 'transition', multiline: true },
  { path: 'gm_notes', multiline: true },
]

const MOODS: MusicMood[] = ['calm', 'exploration', 'tension', 'mystery', 'combat', 'epic']
const OUTCOMES = ['success', 'failure', 'natural_1', 'natural_20'] as const
const SCENE_IMAGE: MediaKind = 'scene'
const ACT_VIDEO: MediaKind = 'intro'

type Save = (edits: Edit[]) => Promise<void>

/** `value` without its empty strings, lists and missing values: what the server keeps of it. */
function compact<T extends object>(value: T): T {
  return Object.fromEntries(
    Object.entries(value).filter(
      ([, v]) => v !== undefined && v !== null && v !== '' && !(Array.isArray(v) && v.length === 0),
    ),
  ) as T
}

/** Key order aside, the same JSON. */
function same(a: unknown, b: unknown): boolean {
  const stable = (v: unknown): unknown =>
    Array.isArray(v)
      ? v.map(stable)
      : v && typeof v === 'object'
        ? Object.fromEntries(
            Object.entries(v)
              .sort(([x], [y]) => x.localeCompare(y))
              .map(([k, x]) => [k, stable(x)]),
          )
        : v
  return JSON.stringify(stable(a)) === JSON.stringify(stable(b))
}

/** The `set` of a whole list of the scene, when it changed (empty clears it). */
function listEdit<T extends object>(target: string, field: string, before: T[] | undefined, after: T[]): Edit[] {
  const rows = after.map(compact)
  if (same((before ?? []).map(compact), rows)) return []
  return [{ op: 'set', target, field, value: rows.length ? rows : null }]
}

function patch<T>(list: T[], i: number, change: Partial<T>): T[] {
  return list.map((row, j) => (j === i ? { ...row, ...change } : row))
}

function lines(text: string): string[] {
  return text
    .split('\n')
    .map((l) => l.trim())
    .filter(Boolean)
}

/**
 * The scene chosen in the graph, prepared in one place
 * (campaign/edit-scenes-in-one-place): its text, its fight, its visuals
 * and its sound. Every change goes through the story edits, applied
 * whole and checked by the server; the visuals are the media and maps
 * of the campaign, drawn and kept here as on their own pages.
 */
export function SceneSheet({
  campaignId,
  story,
  node,
  version,
  busy,
  onSave,
}: {
  campaignId: string
  story: Story
  node: StoryNode
  /** The campaign's `updatedAt`: a saved change resets the forms to what the server kept. */
  version: string
  busy: boolean
  onSave: Save
}) {
  const { t } = useTranslation()
  const [tab, setTab] = useState<SheetTab>('text')
  const act = story.acts?.find((a) => a.id === node.act)
  return (
    <section className="surface-slab flex flex-col gap-3 p-3.5" aria-label={t('prep.scene.label', { title: node.title })}>
      <header className="flex flex-col gap-0.5">
        <span className="type-label text-mute-soft">
          {t('prep.scene.kicker', { act: act?.title ?? node.act })}
          {node.optional && ` · ${t('prep.review.optional')}`}
        </span>
        <h2 className="type-title text-[17px] text-chalk">{node.title}</h2>
      </header>
      <div role="tablist" aria-label={t('prep.scene.tabs')} className="flex gap-1 border-b border-line pb-2">
        {SHEET_TABS.map((id) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            className={cn(
              'flex-1 rounded-button px-2 py-1.5 text-caption font-semibold',
              tab === id ? 'bg-ivory text-ink' : 'text-chalk hover:bg-surface',
            )}
            onClick={() => setTab(id)}
          >
            {t(`prep.scene.tab.${id}`)}
          </button>
        ))}
      </div>
      <div role="tabpanel" aria-label={t(`prep.scene.tab.${tab}`)} className="flex flex-col gap-3">
        {tab === 'text' && <TextTab key={version} story={story} node={node} busy={busy} onSave={onSave} />}
        {tab === 'combat' && <CombatTab key={version} story={story} node={node} busy={busy} onSave={onSave} />}
        {tab === 'visuals' && (
          <VisualsTab campaignId={campaignId} story={story} node={node} busy={busy} onSave={onSave} />
        )}
        {tab === 'sound' && <SoundTab key={version} node={node} busy={busy} onSave={onSave} />}
      </div>
    </section>
  )
}

/** A titled block of the sheet, with an « add » action. */
function Block({ title, onAdd, children }: { title: string; onAdd?: () => void; children: ReactNode }) {
  const { t } = useTranslation()
  return (
    <fieldset className="flex flex-col gap-2 border-t border-line pt-2">
      <legend className="sr-only">{title}</legend>
      <div className="flex items-center justify-between gap-2">
        <h3 className="type-label text-mute-soft">{title}</h3>
        {onAdd && (
          <Btn aria-label={t('prep.scene.addTo', { what: title })} onClick={onAdd}>
            {t('prep.scene.add')}
          </Btn>
        )}
      </div>
      {children}
    </fieldset>
  )
}

/** One row of a list: its fields, and a way to take it out. */
function Row({ label, onRemove, children }: { label: string; onRemove: () => void; children: ReactNode }) {
  const { t } = useTranslation()
  return (
    <div role="group" aria-label={label} className="flex flex-col gap-1.5 rounded-lg border border-line p-2">
      {children}
      <Btn className="self-end" onClick={onRemove}>
        {t('prep.scene.remove')}
      </Btn>
    </div>
  )
}

function SaveBar({ busy, count, onSave }: { busy: boolean; count: number; onSave: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="sticky bottom-0 flex items-center justify-end gap-2 bg-inherit pt-1">
      {count > 0 && <span className="text-caption text-mute-soft">{t('prep.scene.unsaved')}</span>}
      <Btn main disabled={busy || count === 0} onClick={onSave}>
        {t('prep.review.save')}
      </Btn>
    </div>
  )
}

function Select({
  label,
  value,
  onChange,
  children,
}: {
  label: string
  value: string
  onChange: (v: string) => void
  children: ReactNode
}) {
  return (
    <label className="flex min-w-0 flex-1 flex-col gap-1 text-caption text-mute-soft">
      {label}
      <select className={field} value={value} onChange={(e) => onChange(e.target.value)}>
        {children}
      </select>
    </label>
  )
}

function NumberInput({
  label,
  value,
  min,
  onChange,
}: {
  label: string
  value: number | undefined
  min: number
  onChange: (v: number | undefined) => void
}) {
  // What the GM types, kept as typed: clamping each keystroke would turn « 4 » after a clear into « 14 ».
  const [typed, setTyped] = useState(value === undefined ? '' : String(value))
  const parse = (text: string) => {
    const n = Math.floor(Number(text))
    return text === '' || Number.isNaN(n) ? undefined : Math.max(min, n)
  }
  // A row removed above this one hands it another row's value: show that one.
  const shown = typed === '' || parse(typed) === value ? typed : String(value ?? '')
  return (
    <label className="flex w-24 flex-col gap-1 text-caption text-mute-soft">
      {label}
      <input
        className={field}
        type="number"
        min={min}
        value={shown}
        onChange={(e) => {
          setTyped(e.target.value)
          onChange(parse(e.target.value))
        }}
        onBlur={() => setTyped(value === undefined ? '' : String(value))}
      />
    </label>
  )
}

/** Texte: the scene's words, its planned checks and its exits; then its clues. */
function TextTab({ story, node, busy, onSave }: { story: Story; node: StoryNode; busy: boolean; onSave: Save }) {
  const { t } = useTranslation()
  const { values, setValues, edits } = useEntityFields(node.id, node, TEXT_FIELDS)
  const [checks, setChecks] = useState<PlannedCheck[]>(node.checks ?? [])
  const [exits, setExits] = useState(node.exits ?? [])
  const others = (story.nodes ?? []).filter((n) => n.id !== node.id)
  // The stats the campaign already checks: the rule system's, which the server holds to.
  const stats = [
    ...new Set(
      [
        ...(story.nodes ?? []).flatMap((n) => (n.checks ?? []).map((c) => c.stat)),
        ...(story.clues ?? []).map((c) => (c.check as { stat?: string } | undefined)?.stat),
      ].filter((s): s is string => Boolean(s)),
    ),
  ].sort()
  const all = [
    ...edits,
    ...listEdit(node.id, 'checks', node.checks, checks),
    ...listEdit(node.id, 'exits', node.exits, exits),
  ]
  return (
    <>
      <EntityFields
        fields={TEXT_FIELDS}
        values={values}
        onChange={(path, v) => setValues((cur) => ({ ...cur, [path]: v }))}
      />
      <Block
        title={t('prep.scene.checks')}
        onAdd={() => setChecks((cur) => [...cur, { action: '', stat: stats[0] ?? '', difficulty: 10 }])}
      >
        {checks.length === 0 && <p className="text-caption text-mute-soft">{t('prep.scene.noCheck')}</p>}
        <datalist id={`stats-${node.id}`}>
          {stats.map((s) => (
            <option key={s} value={s} />
          ))}
        </datalist>
        {checks.map((c, i) => (
          <Row
            key={i}
            label={t('prep.scene.checkN', { n: i + 1 })}
            onRemove={() => setChecks((cur) => cur.filter((_, j) => j !== i))}
          >
            <TextField
              label={t('prep.scene.check.action')}
              value={c.action}
              onChange={(v) => setChecks((cur) => patch(cur, i, { action: v }))}
            />
            <div className="flex gap-2">
              <label className="flex min-w-0 flex-1 flex-col gap-1 text-caption text-mute-soft">
                {t('prep.scene.check.stat')}
                <input
                  className={field}
                  list={`stats-${node.id}`}
                  value={c.stat}
                  onChange={(e) => setChecks((cur) => patch(cur, i, { stat: e.target.value.trim() }))}
                />
              </label>
              <NumberInput
                label={t('prep.scene.check.difficulty')}
                value={c.difficulty}
                min={0}
                onChange={(v) => setChecks((cur) => patch(cur, i, { difficulty: v ?? 0 }))}
              />
            </div>
            <details className="text-caption text-mute-soft">
              <summary className="cursor-pointer">{t('prep.scene.check.outcomes')}</summary>
              <div className="mt-1.5 flex flex-col gap-1.5">
                {OUTCOMES.map((k) => (
                  <TextField
                    key={k}
                    label={t(`prep.scene.check.${k}`)}
                    value={c[k] ?? ''}
                    onChange={(v) => setChecks((cur) => patch(cur, i, { [k]: v }))}
                  />
                ))}
              </div>
            </details>
          </Row>
        ))}
      </Block>
      <Block
        title={t('prep.scene.exits')}
        onAdd={
          others.length > 0 ? () => setExits((cur) => [...cur, { to: others[0].id, label: others[0].title }]) : undefined
        }
      >
        {exits.length === 0 && <p className="text-caption text-mute-soft">{t('prep.scene.noExit')}</p>}
        {exits.map((x, i) => (
          <Row
            key={i}
            label={t('prep.scene.exitN', { n: i + 1 })}
            onRemove={() => setExits((cur) => cur.filter((_, j) => j !== i))}
          >
            <Select
              label={t('prep.scene.exit.to')}
              value={x.to}
              onChange={(v) => setExits((cur) => patch(cur, i, { to: v }))}
            >
              {!others.some((n) => n.id === x.to) && <option value={x.to}>{t('prep.scene.unknown', { id: x.to })}</option>}
              {others.map((n) => (
                <option key={n.id} value={n.id}>
                  {n.title}
                </option>
              ))}
            </Select>
            <TextField
              label={t('prep.scene.exit.label')}
              value={x.label}
              onChange={(v) => setExits((cur) => patch(cur, i, { label: v }))}
            />
          </Row>
        ))}
      </Block>
      <SaveBar busy={busy} count={all.length} onSave={() => void onSave(all)} />
      <Clues story={story} node={node} busy={busy} onSave={onSave} />
    </>
  )
}

function Clues({ story, node, busy, onSave }: { story: Story; node: StoryNode; busy: boolean; onSave: Save }) {
  const { t } = useTranslation()
  const clues = (story.clues ?? []).filter((c) => c.node === node.id)
  const revelations = story.revelations ?? []
  const [revelation, setRevelation] = useState(revelations[0]?.id ?? '')
  const [text, setText] = useState('')
  return (
    <Block title={t('prep.review.cluesTitle')}>
      {clues.length === 0 && <p className="text-body text-chalk-soft">{t('prep.review.noClue')}</p>}
      <ul className="flex flex-col gap-1.5">
        {clues.map((c) => (
          <li key={c.id} className="flex flex-col rounded-lg bg-surface px-2.5 py-1.5 text-caption">
            <b className="text-chalk">{c.text}</b>
            <span className="text-mute-soft">→ {nameOf(revelations.find((r) => r.id === c.revelation))}</span>
          </li>
        ))}
      </ul>
      {revelations.length > 0 && (
        <div className="flex flex-col gap-2">
          <Select label={t('prep.review.clueLeadsTo')} value={revelation} onChange={setRevelation}>
            {revelations.map((r) => (
              <option key={r.id} value={r.id}>
                {r.statement}
              </option>
            ))}
          </Select>
          <TextField label={t('prep.review.clueText')} multiline value={text} onChange={setText} />
          <Btn
            className="self-end"
            disabled={busy || !text.trim() || !revelation}
            onClick={() => {
              const value: Entity = { id: freshId('cl_', text, story), revelation, node: node.id, text: text.trim() }
              void onSave([{ op: 'add', kind: 'clue', value }]).then(() => setText(''))
            }}
          >
            {t('prep.review.addClue')}
          </Btn>
        </div>
      )}
    </Block>
  )
}

/** Who may be fought: the campaign's adversaries, and its NPCs that have numbers. */
function fighters(story: Story): { adversaries: Entity[]; npcs: Entity[] } {
  return {
    adversaries: story.adversaries ?? [],
    npcs: (story.npcs ?? []).filter((n) => n.stats),
  }
}

/** Combat: who fights, how, when they break, and what the scene gives. */
function CombatTab({ story, node, busy, onSave }: { story: Story; node: StoryNode; busy: boolean; onSave: Save }) {
  const { t } = useTranslation()
  const original = node.encounter
  const { adversaries, npcs } = fighters(story)
  const known = [...adversaries, ...npcs]
  const [fight, setFight] = useState(Boolean(original))
  const [opponents, setOpponents] = useState<Opponents[]>(original?.opponents ?? [])
  const [tactics, setTactics] = useState((original?.tactics ?? []).join('\n'))
  const [morale, setMorale] = useState(original?.morale ?? [])
  const [onVictory, setOnVictory] = useState(original?.on_victory ?? '')
  const [onDefeat, setOnDefeat] = useState(original?.on_defeat ?? '')
  const [loot, setLoot] = useState<Loot[]>(node.loot ?? [])
  const [xp, setXp] = useState(node.xp ?? [])
  const ships = Boolean(original?.vehicles)

  // What the tab does not edit (a ship battle) is carried over as it is.
  const encounter: Encounter | null = fight
    ? compact({
        ...original,
        opponents: opponents.map((o) => compact({ who: o.who, count: o.count === 1 ? undefined : o.count })),
        tactics: lines(tactics),
        morale: morale.map(compact),
        on_victory: onVictory.trim(),
        on_defeat: onDefeat.trim(),
      })
    : null
  const before = original
    ? compact({ ...original, opponents: original.opponents.map((o) => compact({ ...o, count: o.count === 1 ? undefined : o.count })) })
    : null
  const all: Edit[] = [
    ...(same(before, encounter) ? [] : [{ op: 'set' as const, target: node.id, field: 'encounter', value: encounter }]),
    ...listEdit(node.id, 'loot', node.loot, loot),
    ...listEdit(node.id, 'xp', node.xp, xp),
  ]
  const addOpponent = () => setOpponents((cur) => [...cur, { who: known[0]?.id ?? '', count: 1 }])

  return (
    <>
      <div className="flex flex-wrap items-center gap-2">
        <Btn
          main={fight}
          aria-pressed={fight}
          onClick={() => {
            if (!fight && opponents.length === 0 && known.length > 0) addOpponent()
            setFight(true)
          }}
        >
          {t('prep.scene.combat.yes')}
        </Btn>
        <Btn main={!fight} aria-pressed={!fight} disabled={ships} onClick={() => setFight(false)}>
          {t('prep.scene.combat.no')}
        </Btn>
      </div>
      {ships && <p className="text-caption text-mute-soft">{t('prep.scene.combat.ships')}</p>}
      {fight && (
        <>
          <Block title={t('prep.scene.combat.opponents')} onAdd={known.length > 0 ? addOpponent : undefined}>
            {known.length === 0 && <p className="text-caption text-stat-atk">{t('prep.scene.combat.noAdversary')}</p>}
            {opponents.map((o, i) => (
              <Row
                key={i}
                label={t('prep.scene.combat.opponentN', { n: i + 1 })}
                onRemove={() => setOpponents((cur) => cur.filter((_, j) => j !== i))}
              >
                <div className="flex gap-2">
                  <Select
                    label={t('prep.scene.combat.who')}
                    value={o.who}
                    onChange={(v) => setOpponents((cur) => patch(cur, i, { who: v }))}
                  >
                    {!known.some((k) => k.id === o.who) && (
                      <option value={o.who}>{t('prep.scene.unknown', { id: o.who })}</option>
                    )}
                    {adversaries.length > 0 && (
                      <optgroup label={t('prep.scene.combat.adversaries')}>
                        {adversaries.map((a) => (
                          <option key={a.id} value={a.id}>
                            {nameOf(a)}
                          </option>
                        ))}
                      </optgroup>
                    )}
                    {npcs.length > 0 && (
                      <optgroup label={t('prep.scene.combat.npcs')}>
                        {npcs.map((a) => (
                          <option key={a.id} value={a.id}>
                            {nameOf(a)}
                          </option>
                        ))}
                      </optgroup>
                    )}
                  </Select>
                  <NumberInput
                    label={t('prep.scene.combat.count')}
                    value={o.count ?? 1}
                    min={1}
                    onChange={(v) => setOpponents((cur) => patch(cur, i, { count: v ?? 1 }))}
                  />
                </div>
              </Row>
            ))}
          </Block>
          <Block title={t('prep.scene.combat.how')}>
            <TextField label={t('prep.scene.combat.tactics')} multiline value={tactics} onChange={setTactics} />
          </Block>
          <Block
            title={t('prep.scene.combat.morale')}
            onAdd={() => setMorale((cur) => [...cur, { when: '', then: '' }])}
          >
            {morale.map((m, i) => (
              <Row
                key={i}
                label={t('prep.scene.combat.moraleN', { n: i + 1 })}
                onRemove={() => setMorale((cur) => cur.filter((_, j) => j !== i))}
              >
                <TextField
                  label={t('prep.scene.combat.when')}
                  value={m.when}
                  onChange={(v) => setMorale((cur) => patch(cur, i, { when: v }))}
                />
                <TextField
                  label={t('prep.scene.combat.then')}
                  value={m.then}
                  onChange={(v) => setMorale((cur) => patch(cur, i, { then: v }))}
                />
              </Row>
            ))}
            <TextField label={t('prep.scene.combat.onVictory')} multiline value={onVictory} onChange={setOnVictory} />
            <TextField label={t('prep.scene.combat.onDefeat')} multiline value={onDefeat} onChange={setOnDefeat} />
          </Block>
        </>
      )}
      <Block title={t('prep.scene.loot')} onAdd={() => setLoot((cur) => [...cur, { coins: 10, found: '' }])}>
        {loot.length === 0 && <p className="text-caption text-mute-soft">{t('prep.scene.noLoot')}</p>}
        {loot.map((l, i) => (
          <Row
            key={i}
            label={t('prep.scene.lootN', { n: i + 1 })}
            onRemove={() => setLoot((cur) => cur.filter((_, j) => j !== i))}
          >
            <div className="flex gap-2">
              <Select
                label={t('prep.scene.lootItem')}
                value={l.item ?? ''}
                onChange={(v) => setLoot((cur) => patch(cur, i, { item: v || undefined }))}
              >
                <option value="">{t('prep.scene.noItem')}</option>
                {l.item && !(story.items ?? []).some((x) => x.id === l.item) && (
                  <option value={l.item}>{t('prep.scene.unknown', { id: l.item })}</option>
                )}
                {(story.items ?? []).map((x) => (
                  <option key={x.id} value={x.id}>
                    {nameOf(x)}
                  </option>
                ))}
              </Select>
              <NumberInput
                label={t('prep.scene.coins')}
                value={l.coins}
                min={0}
                onChange={(v) => setLoot((cur) => patch(cur, i, { coins: v }))}
              />
            </div>
            <TextField
              label={t('prep.scene.found')}
              value={l.found ?? ''}
              onChange={(v) => setLoot((cur) => patch(cur, i, { found: v }))}
            />
            <label className="flex items-center gap-2 text-caption text-chalk">
              <input
                type="checkbox"
                checked={Boolean(l.hidden)}
                onChange={(e) => setLoot((cur) => patch(cur, i, { hidden: e.target.checked || undefined }))}
              />
              {t('prep.scene.hidden')}
            </label>
          </Row>
        ))}
      </Block>
      <Block title={t('prep.scene.xp')} onAdd={() => setXp((cur) => [...cur, { amount: 1, reason: '' }])}>
        {xp.map((x, i) => (
          <Row
            key={i}
            label={t('prep.scene.xpN', { n: i + 1 })}
            onRemove={() => setXp((cur) => cur.filter((_, j) => j !== i))}
          >
            <div className="flex gap-2">
              <NumberInput
                label={t('prep.scene.xpAmount')}
                value={x.amount}
                min={0}
                onChange={(v) => setXp((cur) => patch(cur, i, { amount: v ?? 0 }))}
              />
              <div className="min-w-0 flex-1">
                <TextField
                  label={t('prep.scene.xpReason')}
                  value={x.reason}
                  onChange={(v) => setXp((cur) => patch(cur, i, { reason: v }))}
                />
              </div>
            </div>
          </Row>
        ))}
      </Block>
      <SaveBar busy={busy} count={all.length} onSave={() => void onSave(all)} />
    </>
  )
}

/** Visuels: the scene's image, the act's video on its first scene, and the map the fight is played on. */
function VisualsTab({
  campaignId,
  story,
  node,
  busy,
  onSave,
}: {
  campaignId: string
  story: Story
  node: StoryNode
  busy: boolean
  onSave: Save
}) {
  const { t } = useTranslation()
  const [media, setMedia] = useState<GmMediaList | null>(null)
  const [maps, setMaps] = useState<MapListing | null>(null)
  const [reload, setReload] = useState(0)
  const [working, setWorking] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [choice, setChoice] = useState('')

  useEffect(() => {
    let live = true
    Promise.all([fetchGmMedia(campaignId), fetchMaps(campaignId)]).then(
      ([m, l]) => {
        if (!live) return
        setMedia(m)
        setMaps(l)
      },
      () => {
        if (live) setError('load')
      },
    )
    return () => {
      live = false
    }
  }, [campaignId, reload])

  useLiveChanges(campaignId, (topics) => {
    if (topics.includes('desk')) setReload((v) => v + 1)
  })

  async function act(run: () => Promise<unknown>) {
    setWorking(true)
    setError(null)
    try {
      await run()
      setReload((v) => v + 1)
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'action')
    } finally {
      setWorking(false)
    }
  }

  const act_ = story.acts?.find((a) => a.id === node.act)
  const opensAct = (story.nodes ?? []).find((n) => n.act === node.act)?.id === node.id
  const card = (kind: MediaKind, subject: string, name: string) =>
    media && (
      <SubjectCard
        campaignId={campaignId}
        kind={kind}
        name={name}
        assets={media.assets.filter((a) => a.kind === kind && a.subject === subject)}
        disabled={working || busy || !media.plan.configured}
        onAsk={(direction) => act(() => askImage(campaignId, kind, subject, direction))}
        onDecide={(asset, approve) => act(() => decideImage(campaignId, asset, approve))}
      />
    )

  const own = maps?.maps.find((m) => m.map.id === node.map)
  const world = maps?.world.find((w) => w.id === node.map)
  const setMap = (id: string | null) => onSave([{ op: 'set', target: node.id, field: 'map', value: id }])

  return (
    <>
      {error && (
        <p role="alert" className="text-caption text-stat-atk">
          {t(`prep.media.errors.${error}`, {
            defaultValue: t(`maps.errors.${error}`, { defaultValue: t('prep.review.errors.action') }),
          })}
        </p>
      )}
      {!media && !error && <p role="status">{t('prep.loading')}</p>}
      {media && !media.plan.configured && (
        <p className="text-caption text-mute-soft">{t('prep.generate.errors.AI_NOT_CONFIGURED')}</p>
      )}
      <Block title={t('prep.scene.visuals.image')}>{card(SCENE_IMAGE, node.id, node.title)}</Block>
      {opensAct && act_ && (
        <Block title={t('prep.scene.visuals.intro', { act: act_.title })}>{card(ACT_VIDEO, act_.id, act_.title)}</Block>
      )}
      <Block title={t('prep.scene.visuals.map')}>
        {maps && !node.map && <p className="text-caption text-mute-soft">{t('prep.scene.visuals.noMap')}</p>}
        {maps && node.map && !own && !world && (
          <p className="text-caption text-stat-atk">{t('prep.scene.visuals.lostMap', { id: node.map })}</p>
        )}
        {world && (
          <p className="text-body">
            <b>{world.name}</b> · <span className="text-mute-soft">{t('prep.scene.visuals.worldMap')}</span>
          </p>
        )}
        {own && (
          <>
            <p className="text-body">
              <b>{own.map.name}</b> ·{' '}
              <span className={own.validatedAt ? 'text-stat-hp' : 'text-mute-soft'}>
                {own.validatedAt ? t('maps.validated') : t('prep.scene.visuals.draftMap')}
              </span>
            </p>
            <MapPreview campaignId={campaignId} map={own} media={media} />
            <div className="flex flex-wrap gap-1.5">
              {!own.validatedAt && (
                <Btn main disabled={working} onClick={() => void act(() => validateMap(campaignId, own.map.id))}>
                  {t('prep.scene.visuals.approveMap')}
                </Btn>
              )}
              <Link
                className="rounded-button border border-line px-2.5 py-1.5 text-caption font-bold text-chalk"
                to={`/campagnes/${campaignId}/cartes/${own.map.id}`}
              >
                {t('prep.scene.visuals.editMap')}
              </Link>
              {!own.validatedAt && own.node === node.id && (
                <Btn
                  disabled={working || busy}
                  onClick={() => {
                    if (!window.confirm(t('maps.confirmDelete', { name: own.map.name }))) return
                    void act(async () => {
                      await setMap(null)
                      await deleteMap(campaignId, own.map.id)
                    })
                  }}
                >
                  {t('prep.scene.visuals.rejectMap')}
                </Btn>
              )}
            </div>
          </>
        )}
        {node.map && (
          <Btn className="self-start" disabled={busy} onClick={() => void setMap(null)}>
            {t('prep.scene.visuals.unlink')}
          </Btn>
        )}
        {maps && (
          <div className="flex items-end gap-2">
            <Select label={t('prep.scene.visuals.choose')} value={choice} onChange={setChoice}>
              <option value="">—</option>
              {maps.maps.map((m) => (
                <option key={m.map.id} value={m.map.id}>
                  {m.map.name} · {m.validatedAt ? t('maps.validated') : t('maps.draft')}
                </option>
              ))}
              {maps.world.map((w) => (
                <option key={w.id} value={w.id}>
                  {w.name} · {t('prep.scene.visuals.worldMap')}
                </option>
              ))}
            </Select>
            <Btn disabled={busy || !choice || choice === node.map} onClick={() => void setMap(choice)}>
              {t('prep.scene.visuals.use')}
            </Btn>
          </div>
        )}
        <Btn
          className="self-start"
          disabled={working || busy}
          onClick={() =>
            void act(async () => {
              const made = await generateMap(campaignId, node.id)
              await setMap(made.map.id)
            })
          }
        >
          {working ? t('maps.generating') : t('prep.scene.visuals.generateMap')}
        </Btn>
        <p className="text-caption text-mute-soft">{t('maps.generateHint')}</p>
      </Block>
    </>
  )
}

/** The map as it will be drawn, small; an imported image behind its grid. */
function MapPreview({ campaignId, map, media }: { campaignId: string; map: CampaignMap; media: GmMediaList | null }) {
  const { t } = useTranslation()
  const { tileset, atlases } = useTileset(map.map, media, (id) => gmImageUrl(campaignId, id))
  const backdrop = useImage(map.backdrop ? gmBackdropUrl(campaignId, map.map.id) : null)
  return (
    <div aria-label={t('prep.scene.visuals.preview', { name: map.map.name })} className="overflow-hidden rounded-md border border-line">
      <MapCanvas scene={{ map: map.map, tileset, atlases, backdrop, tokens: [], tile: 12 }} className="max-h-64" />
    </div>
  )
}

/** Son: the scene's mood, its sounds and its YouTube tracks, with a test listen. */
function SoundTab({ node, busy, onSave }: { node: StoryNode; busy: boolean; onSave: Save }) {
  const { t } = useTranslation()
  const original: Ambience = node.ambience ?? {}
  const [mood, setMood] = useState(original.mood ?? '')
  const [sounds, setSounds] = useState(original.sounds ?? '')
  const [music, setMusic] = useState<MusicTrack[]>(original.music ?? [])
  const [listening, setListening] = useState<{ index: number; startedAt: string } | null>(null)

  const next = compact({ ...original, mood: mood.trim(), sounds: sounds.trim(), music: music.map(compact) })
  const before = compact({ ...original, music: (original.music ?? []).map(compact) })
  const all: Edit[] = same(before, next)
    ? []
    : [{ op: 'set', target: node.id, field: 'ambience', value: Object.keys(next).length ? next : null }]

  return (
    <>
      <TextField label={t('prep.scene.sound.mood')} value={mood} onChange={setMood} />
      <TextField label={t('prep.scene.sound.sounds')} multiline value={sounds} onChange={setSounds} />
      <Block
        title={t('prep.scene.sound.music')}
        onAdd={() => setMusic((cur) => [...cur, { mood: 'calm', title: '' }])}
      >
        {music.length === 0 && <p className="text-caption text-mute-soft">{t('prep.scene.sound.noMusic')}</p>}
        {music.map((m, i) => {
          const playable = youtubeId(m.url ?? '')
          const query = (m.search || m.title).trim()
          return (
            <Row
              key={i}
              label={t('prep.scene.sound.trackN', { n: i + 1 })}
              onRemove={() => {
                setListening(null)
                setMusic((cur) => cur.filter((_, j) => j !== i))
              }}
            >
              <div className="flex gap-2">
                <Select
                  label={t('prep.scene.sound.trackMood')}
                  value={m.mood}
                  onChange={(v) => setMusic((cur) => patch(cur, i, { mood: v as MusicMood }))}
                >
                  {MOODS.map((x) => (
                    <option key={x} value={x}>
                      {t(`evening.music.mood.${x}`)}
                    </option>
                  ))}
                </Select>
                <div className="min-w-0 flex-[2]">
                  <TextField
                    label={t('prep.scene.sound.title')}
                    value={m.title}
                    onChange={(v) => setMusic((cur) => patch(cur, i, { title: v }))}
                  />
                </div>
              </div>
              <TextField
                label={t('prep.scene.sound.url')}
                value={m.url ?? ''}
                onChange={(v) => setMusic((cur) => patch(cur, i, { url: v.trim() }))}
              />
              <TextField
                label={t('prep.scene.sound.search')}
                value={m.search ?? ''}
                onChange={(v) => setMusic((cur) => patch(cur, i, { search: v }))}
              />
              {m.url && !playable && <p className="text-caption text-stat-atk">{t('prep.scene.sound.notYoutube')}</p>}
              {!m.url && <p className="text-caption text-mute-soft">{t('prep.scene.sound.toChoose')}</p>}
              <div className="flex flex-wrap gap-1.5">
                {query && (
                  <a
                    className="rounded-button border border-line px-2.5 py-1.5 text-caption font-bold text-chalk"
                    href={`https://www.youtube.com/results?search_query=${encodeURIComponent(query)}`}
                    target="_blank"
                    rel="noreferrer"
                  >
                    {t('prep.scene.sound.find')}
                  </a>
                )}
                {playable && (
                  <Btn
                    aria-pressed={listening?.index === i}
                    onClick={() =>
                      setListening((cur) => (cur?.index === i ? null : { index: i, startedAt: new Date().toISOString() }))
                    }
                  >
                    {listening?.index === i ? t('prep.scene.sound.stop') : t('prep.scene.sound.listen')}
                  </Btn>
                )}
              </div>
              {playable && listening?.index === i && (
                <MusicPlayer
                  music={{ url: m.url ?? '', title: m.title, mood: m.mood, startedAt: listening.startedAt }}
                />
              )}
            </Row>
          )
        })}
      </Block>
      <SaveBar busy={busy} count={all.length} onSave={() => void onSave(all)} />
    </>
  )
}
