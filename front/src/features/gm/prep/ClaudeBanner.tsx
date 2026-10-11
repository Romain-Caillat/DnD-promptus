import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { StatusBanner } from '@/components/game/StatusBanner'
import { Btn } from '@/features/gm/live/ui'
import { listClaudeProposals } from '@/lib/prep'
import type { PrepChanges } from './usePrepChanges'

/**
 * Claude's latest write on this campaign (platform/stream-prep-edits):
 * « Claude écrit · sur … » while it writes, then what it wrote, with a
 * way to take it back while nothing covered it. Shown on the review
 * screen and the atlas alike; a clue is named by its scene
 * (`sceneTitle`).
 */
export function ClaudeBanner({
  changes,
  busy,
  sceneTitle,
  onUndo,
}: {
  changes: PrepChanges
  busy: boolean
  sceneTitle: (id: string) => string | undefined
  onUndo: () => void
}) {
  const { t } = useTranslation()
  const { claude } = changes
  if (!claude) return null
  const touch = claude.touch
  const scene = touch?.kind === 'clue' && touch.place ? sceneTitle(touch.place) : undefined
  const place = scene ?? (touch ? touch.name || touch.id : null)
  const text = place
    ? t(claude.live ? 'prep.changes.writing' : 'prep.changes.wrote', { place })
    : t('prep.changes.writingNowhere')
  return (
    <div className="flex flex-wrap items-center gap-2">
      <StatusBanner tone={claude.live ? 'wait' : 'listen'} className="flex-1">
        {text}
      </StatusBanner>
      {claude.undoable && (
        <Btn disabled={busy} onClick={onUndo}>
          {t('prep.changes.undo')}
        </Btn>
      )}
      <Btn onClick={changes.dismiss}>{t('prep.changes.dismiss')}</Btn>
    </div>
  )
}

/**
 * How many of Claude's batches wait on the GM (platform/propose-edits-
 * by-claude), read again when `version` moves (the `proposals` topic).
 */
export function useClaudeWaiting(campaignId: string, version: number): number {
  const [count, setCount] = useState(0)
  useEffect(() => {
    let live = true
    listClaudeProposals(campaignId).then(
      (list) => {
        if (live) setCount(list.filter((p) => p.status === 'pending').length)
      },
      () => {},
    )
    return () => {
      live = false
    }
  }, [campaignId, version])
  return count
}
