import { useCallback, useEffect, useRef, useState } from 'react'
import { fetchTrade, type TradeView } from '@/lib/trade'

/**
 * The trade view (`GET /api/play/…/trade`), fetched again when
 * `refreshKey` moves; `set` takes the server's answer to a gesture. A
 * failed refetch keeps what is on screen.
 */
export function useTrade(campaignId: string, refreshKey: number) {
  const [view, setView] = useState<TradeView | null>(null)
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: Awaited<ReturnType<typeof fetchTrade>> | null
    try {
      next = await fetchTrade(campaignId)
    } catch {
      next = null
    }
    // A failed refetch keeps what is on screen; the next change retries.
    if (request !== latest.current || next === null) return
    setView(next)
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  const set = useCallback((next: TradeView) => {
    latest.current++
    setView(next)
  }, [])

  return { view, set }
}
