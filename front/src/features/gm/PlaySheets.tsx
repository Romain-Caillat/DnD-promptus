import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { CellBar } from '@/components/game/CellBar'
import { Hearts } from '@/components/game/Hearts'
import { signed } from '@/features/play/creator/RuleSteps'
import { Sprite } from '@/features/sprites/Sprite'
import { ApiError } from '@/lib/api'
import {
  adjustSheet,
  declareDeath,
  fetchBoard,
  type Adjustment,
  type Board,
  type BoardSheet,
  type HistoryEntry,
} from '@/lib/sheets'

const inputClass =
  'pixel-field px-2.5 py-1.5 text-body'

const timeFormat = new Intl.DateTimeFormat('fr-FR', { timeStyle: 'short' })

type State = { kind: 'loading' } | { kind: 'error' } | { kind: 'ready'; board: Board }

/** The refusals the GM can act on; anything else reads as a failure. */
const ERRORS = {
  NOT_ENOUGH: 'gm.sheets.errors.notEnough',
  INVALID_ITEM_NAME: 'gm.sheets.errors.itemName',
  CHARACTER_NOT_VALIDATED: 'gm.sheets.errors.notValidated',
  NOT_DOWN: 'gm.sheets.errors.notDown',
} as const

type ErrorKey = (typeof ERRORS)[keyof typeof ERRORS] | 'gm.table.error'

function errorKey(err: unknown): ErrorKey {
  const code = err instanceof ApiError ? err.code : ''
  return code in ERRORS ? ERRORS[code as keyof typeof ERRORS] : 'gm.table.error'
}

/**
 * The characters in play, one card each (gm/adjust-sheets-fast): in one
 * gesture the GM gives XP, takes or gives back hit points, gives gold or
 * an item; the player's phone follows live (`players::touch_character`
 * on the server). Below, the history of every change. Laptop and tablet
 * first: six cards hold on one screen.
 *
 * `refreshKey` moves when the live channel says the table changed.
 */
export function PlaySheets({ campaignId, refreshKey }: { campaignId: string; refreshKey: number }) {
  const { t } = useTranslation()
  const [state, setState] = useState<State>({ kind: 'loading' })
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: State
    try {
      next = { kind: 'ready', board: await fetchBoard(campaignId) }
    } catch {
      next = { kind: 'error' }
    }
    if (request !== latest.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setState((s) => (next.kind === 'error' && s.kind === 'ready' ? s : next))
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  if (state.kind === 'loading') return <p role="status">{t('gm.table.loading')}</p>
  if (state.kind === 'error') return <p role="alert">{t('gm.table.error')}</p>
  const { board } = state
  const names = new Map([
    ...board.fallen.map((f) => [f.characterId, f.name || f.nickname] as const),
    ...board.sheets.map((s) => [s.characterId, s.name || s.nickname] as const),
  ])

  return (
    <section className="flex flex-col gap-4" aria-labelledby="sheets-title">
      <div className="flex flex-col gap-1">
        <h2 id="sheets-title" className="type-title text-heading">
          {t('gm.sheets.title')}
        </h2>
        <p className="text-body text-mute-soft">{t('gm.sheets.lead')}</p>
      </div>
      {!board.rulesKnown && <p role="alert">{t('gm.sheets.rulesUnknown')}</p>}
      {board.sheets.length === 0 ? (
        <p className="text-body text-mute">{t('gm.sheets.empty')}</p>
      ) : (
        <ul className="grid gap-3 md:grid-cols-2 2xl:grid-cols-3">
          {board.sheets.map((sheet) => (
            <SheetCard key={sheet.characterId} campaignId={campaignId} sheet={sheet} board={board} onChanged={load} />
          ))}
        </ul>
      )}
      {board.fallen.length > 0 && <Fallen fallen={board.fallen} />}
      <History entries={board.history} names={names} />
    </section>
  )
}

function SheetCard({
  campaignId,
  sheet,
  board,
  onChanged,
}: {
  campaignId: string
  sheet: BoardSheet
  board: Board
  onChanged: () => Promise<void>
}) {
  const { t } = useTranslation()
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState<ErrorKey | null>(null)
  const [more, setMore] = useState(false)
  const [amount, setAmount] = useState(1)
  const [give, setGive] = useState('')
  const [custom, setCustom] = useState({ name: '', description: '' })
  const [dying, setDying] = useState(false)
  const play = sheet.play
  const name = sheet.name || sheet.nickname

  async function send(adjustment: Adjustment) {
    setBusy(true)
    setFailed(null)
    try {
      await adjustSheet(campaignId, sheet.characterId, adjustment)
      await onChanged()
      return true
    } catch (err) {
      setFailed(errorKey(err))
      return false
    } finally {
      setBusy(false)
    }
  }

  async function die() {
    setBusy(true)
    setFailed(null)
    try {
      await declareDeath(campaignId, sheet.characterId)
      await onChanged()
    } catch (err) {
      setFailed(errorKey(err))
    } finally {
      setBusy(false)
      setDying(false)
    }
  }

  async function giveItem() {
    const ok = await send(
      give === 'other'
        ? { kind: 'giveItem', name: custom.name, description: custom.description }
        : { kind: 'giveItem', item: give },
    )
    if (ok) {
      setGive('')
      setCustom({ name: '', description: '' })
    }
  }

  return (
    <li className="surface-slab flex flex-col gap-3 p-3.5" aria-label={name}>
      <div className="flex items-end gap-3">
        <span className="grid h-16 w-12 flex-none place-items-end justify-center overflow-hidden rounded-none border-2 border-line-strong bg-linear-to-b from-surface-raised to-well">
          {sheet.look && <Sprite look={sheet.look} scale={2} />}
        </span>
        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
          <b className="type-title truncate text-[16px]">{name}</b>
          <small className="truncate text-caption text-mute">
            {[sheet.nickname, sheet.className, play && t('gm.sheets.level', { level: play.level })]
              .filter(Boolean)
              .join(' · ')}
          </small>
        </span>
      </div>
      {!play ? (
        <p className="text-caption text-mute">{t('gm.sheets.noClass')}</p>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-2">
            <Hearts hp={play.hitPoints} max={play.maxHitPoints} px={2} animated={false} />
            <span className="text-caption font-bold text-stat-hp">
              {t('gm.sheets.hp', { hp: play.hitPoints, max: play.maxHitPoints })}
            </span>
          </div>
          <div className="flex flex-col gap-1">
            <CellBar stat="init" value={play.xpBar} max={play.xpBarMax} height={8} animated={false} label={t('gm.sheets.xpBar')} />
            <span className="text-caption text-mute-soft">
              {t('gm.sheets.xp', { xp: play.totalXp })}
              {play.upgradePoints > 0 && ` · ${t('gm.sheets.upgrade', { count: play.upgradePoints })}`}
              {play.resources.map((r) => ` · ${r.amount} ${r.abbr}`).join('')}
            </span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            <Button variant="outline" size="sm" disabled={busy || play.hitPoints === 0} onClick={() => void send({ kind: 'hitPoints', delta: -1 })}>
              {t('gm.sheets.hpDelta', { delta: signed(-1) })}
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={busy || play.hitPoints === play.maxHitPoints}
              onClick={() => void send({ kind: 'hitPoints', delta: 1 })}
            >
              {t('gm.sheets.hpDelta', { delta: signed(1) })}
            </Button>
            <Button variant="outline" size="sm" disabled={busy} onClick={() => void send({ kind: 'xp', delta: 1 })}>
              {t('gm.sheets.xpDelta', { delta: signed(1) })}
            </Button>
            <Button variant="ghost" size="sm" aria-expanded={more} onClick={() => setMore((m) => !m)}>
              {t(more ? 'gm.sheets.less' : 'gm.sheets.more')}
            </Button>
            {/* engine/save-against-death: under any rule the GM may decide a death at 0. */}
            {play.hitPoints === 0 && !dying && (
              <Button variant="outline" size="sm" disabled={busy} onClick={() => setDying(true)}>
                {t('gm.sheets.death')}
              </Button>
            )}
          </div>
          {dying && (
            <div role="alertdialog" aria-label={t('gm.sheets.deathConfirm', { name })} className="flex flex-col gap-2 rounded-button border border-stat-atk p-2.5">
              <p className="text-caption text-chalk-soft">{t('gm.sheets.deathHint')}</p>
              <div className="flex flex-wrap gap-1.5">
                <Button variant="destructive" size="sm" disabled={busy} onClick={() => void die()}>
                  {t('gm.sheets.deathConfirm', { name })}
                </Button>
                <Button variant="ghost" size="sm" disabled={busy} onClick={() => setDying(false)}>
                  {t('gm.sheets.cancel')}
                </Button>
              </div>
            </div>
          )}
          {more && (
            <div className="flex flex-col gap-3 border-t border-line pt-3">
              <label className="flex items-center gap-2 text-caption text-mute-soft">
                {t('gm.sheets.amount')}
                <input
                  type="number"
                  min={1}
                  max={99}
                  value={amount}
                  className={`${inputClass} w-20`}
                  onChange={(e) => setAmount(Math.min(99, Math.max(1, Number(e.target.value) || 1)))}
                />
              </label>
              <div className="flex flex-wrap gap-1.5">
                <Button variant="outline" size="sm" disabled={busy} onClick={() => void send({ kind: 'hitPoints', delta: -amount })}>
                  {t('gm.sheets.hpDelta', { delta: signed(-amount) })}
                </Button>
                <Button variant="outline" size="sm" disabled={busy} onClick={() => void send({ kind: 'hitPoints', delta: amount })}>
                  {t('gm.sheets.hpDelta', { delta: signed(amount) })}
                </Button>
                <Button variant="outline" size="sm" disabled={busy} onClick={() => void send({ kind: 'xp', delta: amount })}>
                  {t('gm.sheets.xpDelta', { delta: signed(amount) })}
                </Button>
                <Button variant="outline" size="sm" disabled={busy || play.totalXp === 0} onClick={() => void send({ kind: 'xp', delta: -amount })}>
                  {t('gm.sheets.xpDelta', { delta: signed(-amount) })}
                </Button>
                {board.resources.flatMap((r) =>
                  [amount, -amount].map((delta) => (
                    <Button
                      key={`${r.id}${delta}`}
                      variant="outline"
                      size="sm"
                      disabled={busy}
                      onClick={() => void send({ kind: 'resource', resource: r.id, delta })}
                    >
                      {t('gm.sheets.resourceDelta', { delta: signed(delta), abbr: r.abbr })}
                    </Button>
                  )),
                )}
              </div>
              <div className="flex flex-col gap-1.5">
                <label className="flex flex-col gap-1 text-caption text-mute-soft">
                  {t('gm.sheets.giveLabel')}
                  <select className={inputClass} value={give} onChange={(e) => setGive(e.target.value)}>
                    <option value="">{t('gm.sheets.pick')}</option>
                    {board.items.map((i) => (
                      <option key={i.id} value={i.id}>
                        {i.name}
                      </option>
                    ))}
                    <option value="other">{t('gm.sheets.other')}</option>
                  </select>
                </label>
                {give === 'other' && (
                  <>
                    <input
                      className={inputClass}
                      aria-label={t('gm.sheets.itemName')}
                      placeholder={t('gm.sheets.itemName')}
                      maxLength={80}
                      value={custom.name}
                      onChange={(e) => setCustom({ ...custom, name: e.target.value })}
                    />
                    <input
                      className={inputClass}
                      aria-label={t('gm.sheets.itemText')}
                      placeholder={t('gm.sheets.itemText')}
                      maxLength={500}
                      value={custom.description}
                      onChange={(e) => setCustom({ ...custom, description: e.target.value })}
                    />
                  </>
                )}
                <Button
                  variant="outline"
                  size="sm"
                  className="self-start"
                  disabled={busy || !give || (give === 'other' && !custom.name.trim())}
                  onClick={() => void giveItem()}
                >
                  {t('gm.sheets.give', { name })}
                </Button>
              </div>
            </div>
          )}
          <div className="flex flex-col gap-1">
            <span className="type-label">{t('gm.sheets.bag')}</span>
            {play.inventory.length === 0 ? (
              <span className="text-caption text-mute">{t('gm.sheets.emptyBag')}</span>
            ) : (
              <ul className="flex flex-col gap-1">
                {play.inventory.map((item) => (
                  <li key={item.key} className="flex items-center gap-2 text-caption">
                    <span className="min-w-0 flex-1 truncate">
                      {item.name}
                      {item.qty > 1 && ` ×${item.qty}`}
                      {item.equipped && <span className="text-mute"> · {t('gm.sheets.equipped')}</span>}
                    </span>
                    {more && (
                      <Button
                        variant="ghost"
                        size="xs"
                        disabled={busy}
                        onClick={() => void send({ kind: 'takeItem', entry: item.key })}
                      >
                        {t('gm.sheets.take')}
                      </Button>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </>
      )}
      {failed && <p role="alert" className="text-caption">{t(failed)}</p>}
    </li>
  )
}

/** What a history line says changed. */
function useChangeText() {
  const { t } = useTranslation()
  return (e: HistoryEntry) => {
    const delta = signed(e.after - e.before)
    switch (e.kind) {
      case 'xp':
        return t('gm.sheets.log.xp', { delta, after: e.after })
      case 'hit_points':
        return t('gm.sheets.log.hp', { delta, after: e.after })
      case 'resource':
        return t('gm.sheets.log.resource', { delta, label: e.label ?? '', after: e.after })
      case 'item':
        return t(e.after > e.before ? 'gm.sheets.log.itemGiven' : 'gm.sheets.log.itemTaken', {
          label: e.label ?? '',
          count: Math.abs(e.after - e.before),
        })
      case 'equip':
        return t(e.after ? 'gm.sheets.log.equipped' : 'gm.sheets.log.unequipped', { label: e.label ?? '' })
      case 'level':
        return t('gm.sheets.log.level', { label: e.label ?? '', after: e.after, delta: e.after - e.before })
      case 'death':
        return t('gm.sheets.log.death', { level: e.before })
      case 'upgrade':
        return t('gm.sheets.log.upgrade', { label: e.label ?? '', before: e.before, after: e.after })
    }
  }
}

/** The dead of the campaign, with their last words (planche « Mourir », moment 7). */
function Fallen({ fallen }: { fallen: Board['fallen'] }) {
  const { t } = useTranslation()
  return (
    <section className="surface-slab flex flex-col gap-2 p-3.5" aria-labelledby="sheets-fallen">
      <h3 id="sheets-fallen" className="type-label text-chalk">
        {t('gm.sheets.fallen')}
      </h3>
      <ul className="flex flex-col gap-2">
        {fallen.map((f) => (
          <li key={f.characterId} className="flex items-start gap-3 text-caption">
            {f.look && (
              <span className="flex-none brightness-75 grayscale">
                <Sprite look={f.look} scale={2} />
              </span>
            )}
            <span className="flex flex-col gap-0.5">
              <b>{t('gm.sheets.fallenLine', { name: f.name, level: f.level, nickname: f.nickname })}</b>
              <span className="text-chalk-soft italic">{f.lastWords ? `« ${f.lastWords} »` : t('gm.sheets.noWords')}</span>
              {f.next && <span className="text-mute">{t(`gm.sheets.next.${f.next}`)}</span>}
            </span>
          </li>
        ))}
      </ul>
    </section>
  )
}

/** The history of the changes, newest first. */
function History({ entries, names }: { entries: HistoryEntry[]; names: Map<string, string> }) {
  const { t } = useTranslation()
  const text = useChangeText()
  return (
    <section className="surface-slab flex flex-col gap-2 p-3.5" aria-labelledby="sheets-history">
      <h3 id="sheets-history" className="type-label text-chalk">
        {t('gm.sheets.history')}
      </h3>
      {entries.length === 0 ? (
        <p className="text-caption text-mute">{t('gm.sheets.noHistory')}</p>
      ) : (
        <ol className="flex flex-col gap-1">
          {entries.map((e) => (
            <li key={e.id} className="flex gap-2 text-caption">
              <time className="flex-none text-mute" dateTime={e.createdAt}>
                {timeFormat.format(new Date(e.createdAt))}
              </time>
              <span>
                <b>{names.get(e.characterId) ?? t('gm.sheets.gone')}</b> · {text(e)}
                {e.actor === 'player' && <span className="text-mute"> · {t('gm.sheets.byPlayer')}</span>}
              </span>
            </li>
          ))}
        </ol>
      )}
    </section>
  )
}
