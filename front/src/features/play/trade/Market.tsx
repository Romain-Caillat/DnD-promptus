import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { RollDetail } from '@/components/game/RollDetail'
import { ApiError } from '@/lib/api'
import type { ModifierSource } from '@/lib/rules'
import { buy, haggle, type ShopView, type TradeView } from '@/lib/trade'
import { useTrade } from './useTrade'

/**
 * The shops the GM opened, on the Game tab (player/buy-and-trade): the
 * keeper, what is on the counter at its price now, the purse, and two
 * gestures — buy one unit, and haggle once (the server rolls the
 * shop's check; a won haggle lowers one purchase of the player's
 * choice). What is under the counter never reaches the phone until the
 * GM or a natural 20 brings it out. Nothing shows while no shop is open.
 */
export function Market({
  campaignId,
  refreshKey,
  seated,
}: {
  campaignId: string
  refreshKey: number
  /** A player with a character in play; a spectator only looks. */
  seated: boolean
}) {
  const { t } = useTranslation()
  const { view, set } = useTrade(campaignId, refreshKey)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  if (!view || view.shops.length === 0) return null

  async function act(call: () => Promise<TradeView>) {
    setBusy(true)
    setError(null)
    try {
      set(await call())
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex flex-col gap-4">
      {view.shops.map((shop) => (
        <Shop
          key={shop.id}
          shop={shop}
          purse={view.purse}
          seated={seated}
          busy={busy}
          onBuy={(line, discounted) => void act(() => buy(campaignId, shop.id, line, discounted))}
          onHaggle={() => void act(() => haggle(campaignId, shop.id))}
        />
      ))}
      {error && (
        <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
          {t(`trade.errors.${error}`, { defaultValue: t('trade.errors.UNEXPECTED') })}
        </p>
      )}
    </div>
  )
}

function Shop({
  shop,
  purse,
  seated,
  busy,
  onBuy,
  onHaggle,
}: {
  shop: ShopView
  purse: TradeView['purse']
  seated: boolean
  busy: boolean
  onBuy: (line: string, discounted: boolean) => void
  onHaggle: () => void
}) {
  const { t } = useTranslation()
  const terms = shop.haggle
  const mine = terms?.mine ?? null
  const keeper = shop.keeper || shop.name
  const sourceName = (s: ModifierSource) => ('id' in s && terms && s.id === terms.ability ? terms.abilityName : '')
  return (
    <section className="surface-slab flex flex-col gap-3 p-3.5" aria-label={shop.name}>
      <header className="flex flex-col gap-0.5">
        <span className="type-label">{t('trade.title')}</span>
        <h2 className="type-title text-[24px]">{shop.name}</h2>
        {shop.keeper && <span className="text-caption text-mute-soft">{t('trade.keptBy', { keeper: shop.keeper })}</span>}
      </header>
      {purse && seated && (
        <p className="text-body font-bold">{t('trade.purse', { amount: purse.amount, abbr: purse.abbr })}</p>
      )}
      {shop.surcharge > 0 && (
        <p className="text-caption text-chalk-soft">
          {t('trade.surcharge', { keeper, amount: shop.surcharge, abbr: shop.abbr })}
        </p>
      )}
      <ul className="flex flex-col gap-1.5">
        {shop.lines.map((line) => {
          const soldOut = line.stock === 0
          return (
            <li key={line.key} className="flex flex-col gap-1.5 pixel-field px-3 py-2.5">
              <span className="flex items-baseline justify-between gap-2">
                <b className="text-body">{line.name}</b>
                <span className="flex-none text-body font-bold">
                  {t('trade.price', { price: line.price, abbr: shop.abbr })}
                </span>
              </span>
              {line.description && <small className="text-caption text-mute-soft">{line.description}</small>}
              {line.stock !== null && (
                <small className="text-caption text-mute">
                  {soldOut ? t('trade.soldOut') : t('trade.stock', { count: line.stock })}
                </small>
              )}
              {seated && !soldOut && (
                <span className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    className="pixel-key pixel-key-dark pixel-cursor py-1 pr-3 pb-2 text-label [--px:2px]"
                    aria-label={t('trade.buyAria', { name: line.name })}
                    disabled={busy}
                    onClick={() => onBuy(line.key, false)}
                  >
                    {t('trade.buy')}
                  </button>
                  {line.discountedPrice !== null && (
                    <button
                      type="button"
                      className="pixel-key pixel-cursor py-1 pr-3 pb-2 text-label [--px:2px]"
                      disabled={busy}
                      onClick={() => onBuy(line.key, true)}
                    >
                      {t('trade.buyDiscounted', { price: line.discountedPrice, abbr: shop.abbr })}
                    </button>
                  )}
                </span>
              )}
            </li>
          )
        })}
      </ul>
      {terms && seated && !mine && (
        <CardButton
          variant="dark"
          title={t('trade.haggle.title')}
          subtitle={
            terms.label
              ? t('trade.haggle.hint', { ability: terms.abilityName, label: terms.label, difficulty: terms.difficulty })
              : t('trade.haggle.hintNoLabel', { ability: terms.abilityName, difficulty: terms.difficulty })
          }
          disabled={busy}
          onClick={onHaggle}
        />
      )}
      {mine && (
        <div className="flex flex-col gap-1.5">
          <RollDetail
            roll={mine.roll}
            bandName={(b) => (b === mine.band ? mine.outcome : t(`evening.band.${b}`))}
            sourceName={sourceName}
            targetName={terms?.label ?? undefined}
          />
          <p className="text-body">
            {mine.band === 'success' || mine.band === 'critical_success'
              ? mine.discountLeft
                ? t('trade.haggle.won', { percent: terms?.discountPercent ?? 0 })
                : t('trade.haggle.spent')
              : mine.band === 'critical_failure'
                ? t('trade.haggle.fumbled')
                : t('trade.haggle.lost')}
          </p>
        </div>
      )}
    </section>
  )
}
