import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { GmShops, Shop, ShopInput } from '@/lib/shop'
import { cn } from '@/lib/utils'
import { Btn, Panel, field } from './ui'

type Draft = ShopInput & { id: string | null }

const EMPTY: Draft = { id: null, name: '', lines: [], haggle: null }
const NO_ITEM = ''

function draftOf(shop: Shop): Draft {
  return { id: shop.id, name: shop.name, lines: shop.lines.map((l) => ({ ...l })), haggle: shop.haggle }
}

/**
 * The GM's shops (player/buy-and-trade): write a shop — items of the
 * rules or named ones, a price, a stock, some under the counter, and
 * whether the table may haggle —, open it to the table (one at a time),
 * bring a line out from under the counter, see who haggled.
 */
export function ShopsPanel({
  data,
  onSave,
  onOpen,
  onReveal,
  onDelete,
}: {
  data: GmShops
  onSave: (id: string | null, input: ShopInput) => Promise<boolean>
  onOpen: (shop: string, open: boolean) => void
  onReveal: (shop: string, line: string) => void
  onDelete: (shop: string) => void
}) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState<Draft | null>(null)
  const abbr = data.currency?.abbr ?? ''
  const nameOf = (item: string | null, name: string) =>
    name || data.items.find((i) => i.id === item)?.name || item || ''

  if (draft) {
    return (
      <ShopForm
        draft={draft}
        data={data}
        onChange={setDraft}
        onCancel={() => setDraft(null)}
        onSave={async () => {
          const { id, ...input } = draft
          if (await onSave(id, input)) setDraft(null)
        }}
      />
    )
  }

  return (
    <Panel title={t('gmLive.shops.title')} actions={<Btn onClick={() => setDraft({ ...EMPTY, lines: [] })}>{t('gmLive.shops.new')}</Btn>}>
      {data.shops.length === 0 && <p className="text-caption text-mute-soft">{t('gmLive.shops.none')}</p>}
      <ul className="flex flex-col gap-2">
        {data.shops.map((s) => (
          <li key={s.id} className={cn('flex flex-col gap-1.5 rounded-lg border px-2.5 py-2', s.open ? 'border-chalk' : 'border-line')}>
            <div className="flex items-center justify-between gap-2">
              <b className="text-body">
                {s.name}
                {s.open && <span className="font-normal text-mute-soft"> · {t('gmLive.shops.isOpen')}</span>}
              </b>
              <span className="flex gap-1.5">
                <Btn main={!s.open} onClick={() => onOpen(s.id, !s.open)}>
                  {s.open ? t('gmLive.shops.close') : t('gmLive.shops.open')}
                </Btn>
                <Btn onClick={() => setDraft(draftOf(s))}>{t('gmLive.shops.edit')}</Btn>
                <Btn onClick={() => onDelete(s.id)}>{t('gmLive.shops.delete')}</Btn>
              </span>
            </div>
            <ul className="flex flex-col gap-1">
              {s.lines.map((l) => {
                const tries = s.haggles.filter((h) => h.line === l.id)
                return (
                  <li key={l.id} className="flex items-center justify-between gap-2 text-caption">
                    <span className={cn(l.hidden && 'text-mute-soft')}>
                      {nameOf(l.item, l.name)} · {l.price} {abbr}
                      {l.stock !== null && ` · ${t('gmLive.shops.stock', { count: l.stock })}`}
                      {tries.length > 0 &&
                        ` · ${t('gmLive.shops.haggled', { won: tries.filter((h) => h.success).length, count: tries.length })}`}
                    </span>
                    {l.hidden && (
                      <Btn onClick={() => onReveal(s.id, l.id)} aria-label={`${t('gmLive.shops.reveal')} · ${nameOf(l.item, l.name)}`}>
                        {t('gmLive.shops.reveal')}
                      </Btn>
                    )}
                  </li>
                )
              })}
            </ul>
          </li>
        ))}
      </ul>
    </Panel>
  )
}

function ShopForm({
  draft,
  data,
  onChange,
  onCancel,
  onSave,
}: {
  draft: Draft
  data: GmShops
  onChange: (d: Draft) => void
  onCancel: () => void
  onSave: () => void
}) {
  const { t } = useTranslation()
  const setLine = (i: number, patch: Partial<Draft['lines'][number]>) =>
    onChange({ ...draft, lines: draft.lines.map((l, j) => (j === i ? { ...l, ...patch } : l)) })
  return (
    <Panel title={draft.id ? t('gmLive.shops.editTitle') : t('gmLive.shops.newTitle')}>
      <form
        className="flex flex-col gap-2"
        onSubmit={(e) => {
          e.preventDefault()
          onSave()
        }}
      >
        <input
          aria-label={t('gmLive.shops.name')}
          placeholder={t('gmLive.shops.name')}
          value={draft.name}
          onChange={(e) => onChange({ ...draft, name: e.target.value })}
          className={field}
        />
        {draft.lines.map((l, i) => (
          <fieldset key={l.id ?? `new-${i}`} className="flex flex-col gap-1.5 rounded-lg border border-line p-2">
            <legend className="sr-only">{t('gmLive.shops.line', { n: i + 1 })}</legend>
            <select
              aria-label={t('gmLive.shops.item', { n: i + 1 })}
              value={l.item ?? NO_ITEM}
              onChange={(e) => {
                const item = data.items.find((x) => x.id === e.target.value)
                setLine(i, { item: item?.id ?? null, price: item?.price ?? l.price })
              }}
              className={field}
            >
              <option value={NO_ITEM}>{t('gmLive.shops.named')}</option>
              {data.items.map((x) => (
                <option key={x.id} value={x.id}>
                  {x.name}
                </option>
              ))}
            </select>
            {l.item === null && (
              <>
                <input
                  aria-label={t('gmLive.shops.lineName', { n: i + 1 })}
                  placeholder={t('gmLive.shops.lineNamePlaceholder')}
                  value={l.name}
                  onChange={(e) => setLine(i, { name: e.target.value })}
                  className={field}
                />
                <input
                  aria-label={t('gmLive.shops.lineText', { n: i + 1 })}
                  placeholder={t('gmLive.shops.lineTextPlaceholder')}
                  value={l.description}
                  onChange={(e) => setLine(i, { description: e.target.value })}
                  className={field}
                />
              </>
            )}
            <div className="flex flex-wrap items-center gap-2 text-caption">
              <label className="flex items-center gap-1">
                {t('gmLive.shops.price')}
                <input
                  type="number"
                  min={0}
                  value={l.price}
                  onChange={(e) => setLine(i, { price: Math.max(0, Math.floor(Number(e.target.value) || 0)) })}
                  className={cn(field, 'w-20')}
                />
              </label>
              <label className="flex items-center gap-1">
                {t('gmLive.shops.stockLabel')}
                <input
                  type="number"
                  min={0}
                  placeholder={t('gmLive.shops.unlimited')}
                  value={l.stock ?? ''}
                  onChange={(e) =>
                    setLine(i, { stock: e.target.value === '' ? null : Math.max(0, Math.floor(Number(e.target.value))) })
                  }
                  className={cn(field, 'w-20')}
                />
              </label>
              <label className="flex items-center gap-1">
                <input type="checkbox" checked={l.hidden} onChange={(e) => setLine(i, { hidden: e.target.checked })} />
                {t('gmLive.shops.hidden')}
              </label>
              <Btn onClick={() => onChange({ ...draft, lines: draft.lines.filter((_, j) => j !== i) })}>
                {t('gmLive.shops.removeLine')}
              </Btn>
            </div>
          </fieldset>
        ))}
        <Btn
          className="self-start"
          onClick={() =>
            onChange({
              ...draft,
              lines: [...draft.lines, { item: null, name: '', description: '', price: 1, stock: null, hidden: false }],
            })
          }
        >
          {t('gmLive.shops.addLine')}
        </Btn>
        <label className="flex items-center gap-1.5 text-caption">
          <input
            type="checkbox"
            checked={draft.haggle !== null}
            onChange={(e) =>
              onChange({
                ...draft,
                haggle: e.target.checked ? { ability: data.abilities[0]?.[0] ?? '', difficulty: 12, discount: 20 } : null,
              })
            }
          />
          {t('gmLive.shops.haggle')}
        </label>
        {draft.haggle && (
          <div className="flex flex-wrap items-center gap-2 text-caption">
            <select
              aria-label={t('gmLive.shops.haggleAbility')}
              value={draft.haggle.ability}
              onChange={(e) => onChange({ ...draft, haggle: { ...draft.haggle!, ability: e.target.value } })}
              className={field}
            >
              {data.abilities.map(([id, name]) => (
                <option key={id} value={id}>
                  {name}
                </option>
              ))}
            </select>
            <label className="flex items-center gap-1">
              {t('gmLive.shops.difficulty')}
              <input
                type="number"
                min={1}
                max={99}
                value={draft.haggle.difficulty}
                onChange={(e) => onChange({ ...draft, haggle: { ...draft.haggle!, difficulty: Number(e.target.value) } })}
                className={cn(field, 'w-16')}
              />
            </label>
            <label className="flex items-center gap-1">
              {t('gmLive.shops.discount')}
              <input
                type="number"
                min={1}
                max={90}
                value={draft.haggle.discount}
                onChange={(e) => onChange({ ...draft, haggle: { ...draft.haggle!, discount: Number(e.target.value) } })}
                className={cn(field, 'w-16')}
              />
            </label>
          </div>
        )}
        <div className="flex gap-2">
          <Btn main type="submit" disabled={draft.name.trim() === ''}>
            {t('gmLive.shops.save')}
          </Btn>
          <Btn onClick={onCancel}>{t('gmLive.shops.cancel')}</Btn>
        </div>
      </form>
    </Panel>
  )
}
