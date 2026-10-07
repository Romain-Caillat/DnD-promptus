import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { ApiError } from '@/lib/api'
import type { CharacterView, PlayView } from '@/lib/play'
import { fetchParty, give, type Friend } from '@/lib/shop'

const COINS = 'coins'
const field = 'rounded-button border border-line bg-well px-2 py-1.5 text-body text-chalk'

/**
 * Share with the party (player/buy-and-trade): coins or a line of my
 * bag, to another character in play. The server moves it and tells the
 * table.
 */
export function SharePanel({
  campaignId,
  play,
  onChanged,
}: {
  campaignId: string
  play: PlayView
  onChanged: (character: CharacterView) => void
}) {
  const { t } = useTranslation()
  const [party, setParty] = useState<Friend[]>([])
  const [to, setTo] = useState('')
  const [what, setWhat] = useState(COINS)
  const [amount, setAmount] = useState(1)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const latest = useRef(0)

  useEffect(() => {
    const request = ++latest.current
    void (async () => {
      let next: Friend[] | null
      try {
        next = await fetchParty(campaignId)
      } catch {
        next = null
      }
      if (request !== latest.current || !next) return
      const friends = next
      setParty(friends)
      setTo((current) => current || (friends[0]?.id ?? ''))
    })()
  }, [campaignId])

  const purse = play.resources[0]
  if (party.length === 0) return null
  const line = play.inventory.find((i) => i.key === what)
  const max = what === COINS ? (purse?.amount ?? 0) : (line?.qty ?? 0)

  async function send() {
    setBusy(true)
    setError(null)
    try {
      onChanged(
        await give(campaignId, what === COINS ? { to, coins: amount } : { to, entry: what, qty: amount }),
      )
      setAmount(1)
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="flex flex-col gap-2" aria-label={t('shop.share.title')}>
      <h2 className="type-label">{t('shop.share.title')}</h2>
      <div className="flex flex-wrap items-center gap-2">
        <select aria-label={t('shop.share.to')} value={to} onChange={(e) => setTo(e.target.value)} className={field}>
          {party.map((f) => (
            <option key={f.id} value={f.id}>
              {t('shop.share.friend', { name: f.name, nickname: f.nickname })}
            </option>
          ))}
        </select>
        <select
          aria-label={t('shop.share.what')}
          value={what}
          onChange={(e) => {
            setWhat(e.target.value)
            setAmount(1)
          }}
          className={field}
        >
          {purse && <option value={COINS}>{purse.name}</option>}
          {play.inventory.map((i) => (
            <option key={i.key} value={i.key}>
              {i.name}
            </option>
          ))}
        </select>
        <input
          type="number"
          aria-label={t('shop.share.amount')}
          min={1}
          max={Math.max(1, max)}
          value={amount}
          onChange={(e) => setAmount(Math.max(1, Math.floor(Number(e.target.value) || 1)))}
          className={`${field} w-20`}
        />
        <Button size="sm" disabled={busy || !to || amount > max} onClick={() => void send()}>
          {t('shop.share.give')}
        </Button>
      </div>
      {error && (
        <p role="alert" className="text-caption text-stat-atk">
          {t(`shop.errors.${error}`, { defaultValue: t('shop.errors.UNEXPECTED') })}
        </p>
      )}
    </section>
  )
}
