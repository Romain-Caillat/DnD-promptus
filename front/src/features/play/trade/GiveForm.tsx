import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { ApiError } from '@/lib/api'
import type { PlayView } from '@/lib/play'
import { give } from '@/lib/trade'
import { useTrade } from './useTrade'

const MONEY = '__money__'
const field = 'pixel-field px-2 py-2 text-body'

/**
 * « Partager le butin » (player/buy-and-trade): hand some of a bag line,
 * or money, to a companion in play. The server moves it from one sheet
 * to the other (`POST /api/play/…/character/give`); both sheets follow
 * on the live channel, and the table's journal says who gave what.
 */
export function GiveForm({ campaignId, play }: { campaignId: string; play: PlayView }) {
  const { t } = useTranslation()
  const { view, set } = useTrade(campaignId, 0)
  const [what, setWhat] = useState('')
  const [to, setTo] = useState('')
  const [qty, setQty] = useState('1')
  const [state, setState] = useState<{ kind: 'idle' } | { kind: 'busy' } | { kind: 'done' } | { kind: 'error'; code: string }>({
    kind: 'idle',
  })
  if (!view) return null
  const purse = view.purse
  const line = play.inventory.find((i) => i.key === what)
  const max = what === MONEY ? (purse?.amount ?? 0) : (line?.qty ?? 0)
  const count = Math.max(1, Math.min(max, Math.floor(Number(qty)) || 1))

  async function send() {
    setState({ kind: 'busy' })
    try {
      set(await give(campaignId, what === MONEY ? { to, amount: count } : { to, entry: what, qty: count }))
      setState({ kind: 'done' })
      setWhat('')
      setQty('1')
    } catch (err) {
      setState({ kind: 'error', code: err instanceof ApiError ? err.code : 'UNEXPECTED' })
    }
  }

  return (
    <section className="flex flex-col gap-2" aria-label={t('trade.give.title')}>
      <h2 className="type-label">{t('trade.give.title')}</h2>
      {view.companions.length === 0 ? (
        <p className="text-caption text-mute">{t('trade.give.none')}</p>
      ) : (
        <div className="flex flex-col gap-2">
          <label className="flex flex-col gap-1 text-caption text-mute-soft">
            {t('trade.give.what')}
            <select className={field} value={what} onChange={(e) => setWhat(e.target.value)}>
              <option value="" />
              {purse && purse.amount > 0 && <option value={MONEY}>{t('trade.give.money', { name: purse.name })}</option>}
              {play.inventory.map((i) => (
                <option key={i.key} value={i.key}>
                  {i.qty > 1 ? `${i.name} ×${i.qty}` : i.name}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-caption text-mute-soft">
            {t('trade.give.to')}
            <select className={field} value={to} onChange={(e) => setTo(e.target.value)}>
              <option value="" />
              {view.companions.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
          </label>
          {max > 1 && (
            <label className="flex flex-col gap-1 text-caption text-mute-soft">
              {t('trade.give.qty')}
              <input
                type="number"
                className={field}
                min={1}
                max={max}
                value={qty}
                onChange={(e) => setQty(e.target.value)}
              />
            </label>
          )}
          <Button disabled={!what || !to || state.kind === 'busy'} onClick={() => void send()}>
            {t('trade.give.send')}
          </Button>
          {state.kind === 'done' && <p role="status" className="text-caption">{t('trade.give.done')}</p>}
          {state.kind === 'error' && (
            <p role="alert" className="text-caption">
              {t(`trade.errors.${state.code}`, { defaultValue: t('trade.errors.UNEXPECTED') })}
            </p>
          )}
        </div>
      )}
    </section>
  )
}
