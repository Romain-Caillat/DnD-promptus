import { useCallback, useEffect, useRef, useState } from 'react'
import { fetchBetween, type BetweenView } from '@/lib/between'

export type BetweenState = { kind: 'loading' } | { kind: 'error' } | { kind: 'ready'; between: BetweenView }

/**
 * The between screen's data (`GET /api/play/…/between`), fetched again
 * whenever `refreshKey` moves (the live channel said the session or the
 * character changed). A failed refetch keeps what is on screen.
 */
export function useBetween(campaignId: string, refreshKey: number): BetweenState {
  const [state, setState] = useState<BetweenState>({ kind: 'loading' })
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: BetweenState
    try {
      next = { kind: 'ready', between: await fetchBetween(campaignId) }
    } catch {
      next = { kind: 'error' }
    }
    if (request !== latest.current) return
    setState((s) => (next.kind === 'error' && s.kind === 'ready' ? s : next))
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  return state
}
