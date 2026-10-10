import { useCallback, useEffect, useRef, useState } from 'react'
import type { LiveActivity } from '@/lib/live'
import { fetchRevisions, undoRevision, type ReviewCampaign, type RevisionEntry, type RevisionTouch } from '@/lib/prep'

/** How long what someone else changed stays lit, and Claude "writing". */
const HIGHLIGHT_MS = 8_000

/** Claude's latest write, as the banner shows it. */
interface ClaudeWrite {
  revision: number
  /** What it touched first; `null` until the journal is read. */
  touch: RevisionTouch | null
  /** Heard within `HIGHLIGHT_MS`: Claude is writing now. */
  live: boolean
  undoable: boolean
}

export interface PrepChanges {
  /** Ids of the entities someone other than this GM changed just now. */
  fresh: ReadonlySet<string>
  claude: ClaudeWrite | null
  /** Give to `useLiveChanges`: the topics that moved. */
  onTopics: (topics: string[]) => void
  /** Give to `useLiveChanges`: Claude's activity. */
  onActivity: (activity: LiveActivity) => void
  /** Take Claude's latest write back; resolves to the campaign after. */
  undo: () => Promise<ReviewCampaign | null>
  dismiss: () => void
}

/**
 * platform/stream-prep-edits — what moved in the preparation while the
 * GM's screen is open. On each new revision (the GM-only `prep` topic)
 * it reads the journal since the revision it held and lights up what
 * Claude or the co-GM touched; Claude's activity shows at once, named
 * once the journal is read. `revision` is the campaign's as loaded,
 * `null` before.
 */
export function usePrepChanges(campaignId: string, revision: number | null): PrepChanges {
  const held = useRef<number | null>(null)
  const [fresh, setFresh] = useState<ReadonlySet<string>>(new Set())
  const [claude, setClaude] = useState<ClaudeWrite | null>(null)
  const freshTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
  const liveTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  useEffect(() => {
    if (held.current === null && revision !== null) held.current = revision
  }, [revision])

  useEffect(
    () => () => {
      clearTimeout(freshTimer.current)
      clearTimeout(liveTimer.current)
    },
    [],
  )

  const markLive = useCallback(() => {
    clearTimeout(liveTimer.current)
    liveTimer.current = setTimeout(() => setClaude((c) => (c ? { ...c, live: false } : c)), HIGHLIGHT_MS)
  }, [])

  const catchUp = useCallback(async () => {
    const since = held.current
    if (since === null) return
    let read: { revision: number; entries: RevisionEntry[] }
    try {
      read = await fetchRevisions(campaignId, since)
    } catch {
      return
    }
    held.current = Math.max(held.current ?? since, read.revision)
    // A clue lights its scene too, even once removed.
    const ids = read.entries
      .filter((e) => e.author !== 'gm')
      .flatMap((e) => e.changes.flatMap((c) => (c.kind === 'clue' && c.place ? [c.id, c.place] : [c.id])))
    if (ids.length > 0) {
      setFresh(new Set(ids))
      clearTimeout(freshTimer.current)
      freshTimer.current = setTimeout(() => setFresh(new Set()), HIGHLIGHT_MS)
    }
    const last = [...read.entries].reverse().find((e) => e.author === 'token')
    if (last) {
      setClaude((c) => ({
        revision: last.revision,
        touch: last.changes[0] ?? null,
        live: c?.revision === last.revision ? c.live : true,
        undoable: last.undoable,
      }))
    } else {
      // A later write may have covered Claude's: say so.
      setClaude((c) => (c && read.entries.length > 0 ? { ...c, undoable: false } : c))
    }
  }, [campaignId])

  const onTopics = useCallback(
    (topics: string[]) => {
      if (topics.includes('prep')) void catchUp()
    },
    [catchUp],
  )

  const onActivity = useCallback(
    (activity: LiveActivity) => {
      // The journal may already have named it (the change comes first).
      setClaude((c) =>
        c?.revision === activity.revision
          ? { ...c, live: true }
          : { revision: activity.revision, touch: null, live: true, undoable: false },
      )
      markLive()
    },
    [markLive],
  )

  const undo = useCallback(async () => {
    if (!claude) return null
    const campaign = await undoRevision(campaignId, claude.revision)
    setClaude(null)
    return campaign
  }, [campaignId, claude])

  const dismiss = useCallback(() => setClaude(null), [])

  return { fresh, claude, onTopics, onActivity, undo, dismiss }
}
