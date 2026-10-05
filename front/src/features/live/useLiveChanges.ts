import { useEffect, useRef, useState } from 'react'
import { LiveConnection, liveUrl, type LiveStatus, type Presence } from '@/lib/live'

export interface LiveState {
  status: LiveStatus
  presence: Presence
}

const NOBODY: Presence = { gmOnline: false, players: [] }

/**
 * Follow the live channel of `campaignId` while the component is
 * mounted. `onChange` receives the topics that moved (`world`, `story`,
 * `character:<id>`…) and refetches them through the API — on each
 * change, and after a reconnection for everything missed meanwhile.
 */
export function useLiveChanges(
  campaignId: string,
  onChange: (topics: string[]) => void,
): LiveState {
  const [status, setStatus] = useState<LiveStatus>('connecting')
  const [presence, setPresence] = useState<Presence>(NOBODY)
  const onChangeRef = useRef(onChange)
  useEffect(() => {
    onChangeRef.current = onChange
  })

  useEffect(() => {
    const connection = new LiveConnection({
      url: liveUrl(campaignId),
      onChange: (topics) => onChangeRef.current(topics),
      onPresence: setPresence,
      onStatus: setStatus,
    })
    connection.start()
    return () => connection.stop()
  }, [campaignId])

  return { status, presence }
}
