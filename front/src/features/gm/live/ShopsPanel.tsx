import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ApiError } from '@/lib/api'
import {
  createShop,
  deleteShop,
  fetchShops,
  revealLine,
  saveShop,
  setShopOpen,
  type GmShop,
  type Haggle,
  type ShopLine,
  type ShopsScreen,
} from '@/lib/trade'
import { Btn, Panel, field } from './ui'

/**
 * The shops of the campaign on the GM's live screen (player/buy-and-trade):
 * open the shop of an NPC who sells (Dents-de-Fer: his prices, the
 * compass he hides, the haggling check of his scene) or an empty one;
 * edit the counter — an item of the rules or the story, or one the GM
 * names; price, stock, under the counter — and the haggling terms; open
 * it to the players, bring out a hidden line, read who haggled and how.
 * The server holds every price and roll (`GET /api/campaigns/…/shops`).
 */
export function ShopsPanel({ campaignId, refreshKey }: { campaignId: string; refreshKey: number }) {
  const { t } = useTranslation()
  const [screen, setScreen] = useState<ShopsScreen | null>(null)
  const [editing, setEditing] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [name, setName] = useState('')
  const [keeper, setKeeper] = useState('')
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: Awaited<ReturnType<typeof fetchShops>> | null
    try {
      next = await fetchShops(campaignId)
    } catch {
      next = null
    }
    // A failed refetch keeps what is on screen; the next change retries.
    if (request !== latest.current || next === null) return
    setScreen(next)
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  async function act(call: () => Promise<ShopsScreen>) {
    setError(null)
    try {
      latest.current++
      setScreen(await call())
      return true
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
      return false
    }
  }

  if (!screen) return null
  const taken = new Set(screen.shops.map((s) => s.npc))

  return (
    <Panel title={t('gmShops.title')}>
      {!screen.currency ? (
        <p className="text-caption text-chalk-soft">{t('gmShops.noCurrency')}</p>
      ) : (
        <div className="flex flex-wrap items-end gap-1.5">
          {screen.keepers
            .filter((k) => !taken.has(k.id))
            .map((k) => (
              <Btn key={k.id} onClick={() => void act(() => createShop(campaignId, { fromNpc: k.id }))}>
                {t('gmShops.fromNpc', { name: k.title || k.name })}
              </Btn>
            ))}
          <input
            className={field}
            placeholder={t('gmShops.blankName')}
            aria-label={t('gmShops.blankName')}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <input
            className={field}
            placeholder={t('gmShops.keeper')}
            aria-label={t('gmShops.keeper')}
            value={keeper}
            onChange={(e) => setKeeper(e.target.value)}
          />
          <Btn
            disabled={!name.trim()}
            onClick={() =>
              void act(() => createShop(campaignId, { name, keeper })).then((ok) => {
                if (ok) {
                  setName('')
                  setKeeper('')
                }
              })
            }
          >
            {t('gmShops.create')}
          </Btn>
        </div>
      )}
      {screen.shops.length === 0 && <p className="text-caption text-mute">{t('gmShops.none')}</p>}
      {screen.shops.map((shop) =>
        editing === shop.id ? (
          <ShopEditor
            key={shop.id}
            shop={shop}
            screen={screen}
            onCancel={() => setEditing(null)}
            onSave={(edit) =>
              void act(() => saveShop(campaignId, shop.id, edit)).then((ok) => {
                if (ok) setEditing(null)
              })
            }
          />
        ) : (
          <ShopCard
            key={shop.id}
            shop={shop}
            abbr={screen.currency?.abbr ?? ''}
            onOpen={(open) => void act(() => setShopOpen(campaignId, shop.id, open))}
            onReveal={(line) => void act(() => revealLine(campaignId, shop.id, line))}
            onEdit={() => setEditing(shop.id)}
            onDelete={() => void act(() => deleteShop(campaignId, shop.id))}
          />
        ),
      )}
      {error && (
        <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-caption">
          {t(`gmShops.errors.${error}`, { defaultValue: t('gmShops.errors.UNEXPECTED', { code: error }) })}
        </p>
      )}
    </Panel>
  )
}

function ShopCard({
  shop,
  abbr,
  onOpen,
  onReveal,
  onEdit,
  onDelete,
}: {
  shop: GmShop
  abbr: string
  onOpen: (open: boolean) => void
  onReveal: (line: string) => void
  onEdit: () => void
  onDelete: () => void
}) {
  const { t } = useTranslation()
  return (
    <article className="flex flex-col gap-2 rounded-button border border-line p-2.5" aria-label={shop.name}>
      <header className="flex flex-wrap items-center justify-between gap-2">
        <span className="text-body">
          <b>{shop.name}</b>
          {shop.keeper && <span className="text-mute-soft"> · {shop.keeper}</span>}
          <span className="text-caption text-mute"> · {t(shop.open ? 'gmShops.isOpen' : 'gmShops.isClosed')}</span>
        </span>
        <span className="flex gap-1.5">
          <Btn main={!shop.open} onClick={() => onOpen(!shop.open)}>
            {t(shop.open ? 'gmShops.close' : 'gmShops.open')}
          </Btn>
          <Btn onClick={onEdit}>{t('gmShops.edit')}</Btn>
          <Btn onClick={onDelete}>{t('gmShops.delete')}</Btn>
        </span>
      </header>
      <ul className="flex flex-col gap-1 text-caption">
        {shop.lines.map((l) => (
          <li key={l.key} className="flex items-center justify-between gap-2">
            <span>
              {l.displayName} · {l.price} {abbr}
              {l.stock !== null && l.stock !== undefined && ` · ${t('gmShops.stock')} ${l.stock}`}
              {l.hidden && <span className="text-mute"> · {t('gmShops.hidden')}</span>}
            </span>
            {l.hidden && (
              <Btn aria-label={t('gmShops.revealAria', { name: l.displayName })} onClick={() => onReveal(l.key)}>
                {t('gmShops.reveal')}
              </Btn>
            )}
          </li>
        ))}
      </ul>
      {shop.surcharge > 0 && (
        <p className="text-caption text-chalk-soft">{t('gmShops.surcharge', { amount: shop.surcharge })}</p>
      )}
      {shop.haggles.length > 0 && (
        <div className="flex flex-col gap-0.5 text-caption">
          <span className="type-label">{t('gmShops.haggles')}</span>
          {shop.haggles.map((h) => (
            <span key={h.characterId}>
              {t('gmShops.haggleLine', { name: h.name, outcome: h.outcome, total: h.total })}
              {h.discountLeft && <span className="text-mute"> · {t('gmShops.discountLeft')}</span>}
            </span>
          ))}
        </div>
      )}
    </article>
  )
}

type EditLine = ShopLine & { label: string }

/** A line as the server takes it: the label is only for the editor. */
function withoutLabel(line: EditLine): ShopLine {
  const copy: Partial<EditLine> = { ...line }
  delete copy.label
  return copy as ShopLine
}

function ShopEditor({
  shop,
  screen,
  onSave,
  onCancel,
}: {
  shop: GmShop
  screen: ShopsScreen
  onSave: (edit: { name: string; keeper: string; lines: ShopLine[]; haggle: Haggle | null }) => void
  onCancel: () => void
}) {
  const { t } = useTranslation()
  const [name, setName] = useState(shop.name)
  const [keeper, setKeeper] = useState(shop.keeper)
  const [lines, setLines] = useState<EditLine[]>(() =>
    shop.lines.map(({ displayName, ...l }) => ({ ...l, label: displayName })),
  )
  const [haggle, setHaggle] = useState<Haggle | null>(shop.haggle)
  const [pick, setPick] = useState('')
  const [named, setNamed] = useState('')
  const abilities = screen.abilities ?? []
  const difficulties = screen.difficulties ?? []
  const patch = (i: number, p: Partial<ShopLine>) => setLines((ls) => ls.map((l, j) => (j === i ? { ...l, ...p } : l)))

  function add() {
    const found = screen.sellable.find((s) => `${s.kind}:${s.id}` === pick)
    if (found) {
      const line: EditLine = {
        key: '',
        ...(found.kind === 'rules' ? { item: found.id } : { storyItem: found.id }),
        price: found.price ?? 0,
        stock: null,
        hidden: false,
        label: found.name,
      }
      setLines((ls) => [...ls, line])
      setPick('')
    } else if (named.trim()) {
      const line: EditLine = { key: '', name: named.trim(), price: 0, stock: null, hidden: false, label: named.trim() }
      setLines((ls) => [...ls, line])
      setNamed('')
    }
  }

  return (
    <form
      className="flex flex-col gap-2 rounded-button border border-chalk p-2.5"
      aria-label={shop.name}
      onSubmit={(e) => {
        e.preventDefault()
        onSave({ name, keeper, lines: lines.map(withoutLabel), haggle })
      }}
    >
      <div className="flex flex-wrap gap-1.5">
        <input className={field} aria-label={t('gmShops.name')} value={name} onChange={(e) => setName(e.target.value)} />
        <input
          className={field}
          aria-label={t('gmShops.keeper')}
          placeholder={t('gmShops.keeper')}
          value={keeper}
          onChange={(e) => setKeeper(e.target.value)}
        />
      </div>
      <span className="type-label">{t('gmShops.lines')}</span>
      <ul className="flex flex-col gap-1">
        {lines.map((l, i) => (
          <li key={`${l.key}-${i}`} className="flex flex-wrap items-center gap-1.5 text-caption">
            <span className="min-w-32 flex-1">{l.label}</span>
            <label className="flex items-center gap-1">
              {t('gmShops.price')}
              <input
                type="number"
                min={0}
                className={`${field} w-20`}
                value={l.price}
                onChange={(e) => patch(i, { price: Math.max(0, Number(e.target.value) || 0) })}
              />
            </label>
            <label className="flex items-center gap-1">
              {t('gmShops.stock')}
              <input
                type="number"
                min={0}
                className={`${field} w-16`}
                placeholder={t('gmShops.unlimited')}
                value={l.stock ?? ''}
                onChange={(e) => patch(i, { stock: e.target.value === '' ? null : Math.max(0, Number(e.target.value) || 0) })}
              />
            </label>
            <label className="flex items-center gap-1">
              <input type="checkbox" checked={l.hidden} onChange={(e) => patch(i, { hidden: e.target.checked })} />
              {t('gmShops.hide')}
            </label>
            <Btn onClick={() => setLines((ls) => ls.filter((_, j) => j !== i))}>{t('gmShops.remove')}</Btn>
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap items-center gap-1.5">
        <select className={field} aria-label={t('gmShops.pickItem')} value={pick} onChange={(e) => setPick(e.target.value)}>
          <option value="">{t('gmShops.pickItem')}</option>
          {screen.sellable.map((s) => (
            <option key={`${s.kind}:${s.id}`} value={`${s.kind}:${s.id}`}>
              {s.name}
            </option>
          ))}
        </select>
        <input
          className={field}
          aria-label={t('gmShops.namedItem')}
          placeholder={t('gmShops.namedItem')}
          value={named}
          onChange={(e) => setNamed(e.target.value)}
        />
        <Btn disabled={!pick && !named.trim()} onClick={add}>
          {t('gmShops.addLine')}
        </Btn>
      </div>
      <span className="type-label">{t('gmShops.haggle')}</span>
      <label className="flex items-center gap-1 text-caption">
        <input
          type="checkbox"
          checked={haggle === null}
          onChange={(e) =>
            setHaggle(
              e.target.checked
                ? null
                : {
                    ability: abilities[0]?.id ?? '',
                    difficulty: difficulties[1]?.value ?? difficulties[0]?.value ?? 10,
                    discountPercent: 50,
                    fumbleSurcharge: 0,
                    criticalReveals: false,
                  },
            )
          }
        />
        {t('gmShops.noHaggle')}
      </label>
      {haggle && (
        <div className="flex flex-wrap items-center gap-1.5 text-caption">
          <select
            className={field}
            aria-label={t('gmShops.ability')}
            value={haggle.ability}
            onChange={(e) => setHaggle({ ...haggle, ability: e.target.value })}
          >
            {abilities.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
          <select
            className={field}
            aria-label={t('gmShops.difficulty')}
            value={haggle.difficulty}
            onChange={(e) => setHaggle({ ...haggle, difficulty: Number(e.target.value) })}
          >
            {difficulties.map((d) => (
              <option key={d.value} value={d.value}>
                {d.name} ({d.value})
              </option>
            ))}
          </select>
          <label className="flex items-center gap-1">
            {t('gmShops.discount')}
            <input
              type="number"
              min={0}
              max={100}
              className={`${field} w-16`}
              value={haggle.discountPercent}
              onChange={(e) =>
                setHaggle({ ...haggle, discountPercent: Math.min(100, Math.max(0, Number(e.target.value) || 0)) })
              }
            />
          </label>
          <label className="flex items-center gap-1">
            {t('gmShops.fumble')}
            <input
              type="number"
              min={0}
              className={`${field} w-16`}
              value={haggle.fumbleSurcharge}
              onChange={(e) => setHaggle({ ...haggle, fumbleSurcharge: Math.max(0, Number(e.target.value) || 0) })}
            />
          </label>
          <label className="flex items-center gap-1">
            <input
              type="checkbox"
              checked={haggle.criticalReveals}
              onChange={(e) => setHaggle({ ...haggle, criticalReveals: e.target.checked })}
            />
            {t('gmShops.criticalReveals')}
          </label>
        </div>
      )}
      <div className="flex gap-1.5">
        <Btn main type="submit">
          {t('gmShops.save')}
        </Btn>
        <Btn onClick={onCancel}>{t('gmShops.cancel')}</Btn>
      </div>
    </form>
  )
}
