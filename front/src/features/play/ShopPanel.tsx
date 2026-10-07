import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { ApiError } from '@/lib/api'
import { buy, fetchShop, haggle, type ShopView } from '@/lib/shop'

/**
 * The shop the GM opened (player/buy-and-trade): the counter with my
 * price for each line, my purse, « Acheter », and « Marchander » once per
 * line when the shop allows it — the server rolls. A spectator sees the
 * counter only. Nothing shows while no shop is open.
 */
export function ShopPanel({
  campaignId,
  refreshKey,
  onBought,
}: {
  campaignId: string
  refreshKey: number
  /** Something moved on my sheet. */
  onBought?: () => void
}) {
  const { t } = useTranslation()
  const [shop, setShop] = useState<ShopView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [outcome, setOutcome] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: { shop: ShopView | null } | null
    try {
      next = { shop: await fetchShop(campaignId) }
    } catch {
      next = null
    }
    if (request !== latest.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setShop((s) => (next ? next.shop : s))
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  async function act(call: () => Promise<ShopView | null>) {
    setBusy(true)
    setError(null)
    try {
      ++latest.current
      setShop(await call())
      onBought?.()
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    } finally {
      setBusy(false)
    }
  }

  if (!shop) return null
  const purse = shop.purse
  return (
    <section className="surface-slab flex flex-col gap-2.5 p-3.5" aria-label={shop.name}>
      <header className="flex items-baseline justify-between gap-2">
        <h2 className="type-title text-[20px]">{shop.name}</h2>
        {purse && (
          <span className="text-caption text-chalk-soft">
            {t('shop.purse', { amount: purse.amount, abbr: purse.abbr })}
          </span>
        )}
      </header>
      {shop.haggle && (
        <p className="text-caption text-mute-soft">
          {t('shop.haggleRule', {
            ability: shop.haggle.abilityName,
            difficulty: shop.haggle.difficulty,
            discount: shop.haggle.discount,
          })}
        </p>
      )}
      {shop.lines.length === 0 && <p className="text-caption text-mute">{t('shop.empty')}</p>}
      <ul className="flex flex-col gap-1.5">
        {shop.lines.map((l) => {
          const soldOut = l.stock === 0
          const tooDear = purse !== null && purse.amount < l.price
          return (
            <li key={l.id} className="flex flex-col gap-1.5 rounded-button border border-line bg-well px-3 py-2.5">
              <div className="flex items-baseline justify-between gap-2">
                <b className="text-body">{l.name}</b>
                <span className="text-body whitespace-nowrap">
                  {l.price < l.listPrice && (
                    <s className="mr-1.5 text-caption text-mute">{t('shop.price', { price: l.listPrice, abbr: purse?.abbr ?? '' })}</s>
                  )}
                  {t('shop.price', { price: l.price, abbr: purse?.abbr ?? '' })}
                </span>
              </div>
              {l.description && <small className="text-caption text-mute-soft">{l.description}</small>}
              <div className="flex flex-wrap items-center gap-2">
                {l.stock !== null && (
                  <span className="text-caption text-mute">
                    {soldOut ? t('shop.soldOut') : t('shop.stock', { count: l.stock })}
                  </span>
                )}
                {l.haggled !== null && (
                  <span className="text-caption text-mute">{l.haggled ? t('shop.haggleWon') : t('shop.haggleLost')}</span>
                )}
                {purse && (
                  <span className="ml-auto flex gap-2">
                    {shop.haggle && l.haggled === null && (
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={busy || soldOut}
                        aria-label={`${t('shop.haggle')} · ${l.name}`}
                        onClick={() =>
                          void act(async () => {
                            const r = await haggle(campaignId, l.id)
                            const won = r.roll.band === 'success' || r.roll.band === 'critical_success'
                            setOutcome(
                              t(won ? 'shop.haggledWon' : 'shop.haggledLost', {
                                name: l.name,
                                total: r.roll.total,
                                difficulty: shop.haggle!.difficulty,
                              }),
                            )
                            return r.shop
                          })
                        }
                      >
                        {t('shop.haggle')}
                      </Button>
                    )}
                    <Button
                      size="sm"
                      disabled={busy || soldOut || tooDear}
                      aria-label={`${t('shop.buy')} · ${l.name}`}
                      onClick={() => {
                        setOutcome(null)
                        void act(() => buy(campaignId, l.id))
                      }}
                    >
                      {t('shop.buy')}
                    </Button>
                  </span>
                )}
              </div>
            </li>
          )
        })}
      </ul>
      {outcome && <p role="status" className="text-body">{outcome}</p>}
      {error && (
        <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
          {t(`shop.errors.${error}`, { defaultValue: t('shop.errors.UNEXPECTED') })}
        </p>
      )}
    </section>
  )
}
