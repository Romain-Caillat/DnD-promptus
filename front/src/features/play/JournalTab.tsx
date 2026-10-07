import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { PlayerView } from '@/lib/campaigns'
import { shortDate } from '@/lib/dates'
import { fetchChronicle, type ChronicleEntry } from '@/lib/evening'
import { fetchCampaignView } from '@/lib/play'

type State = { kind: 'loading' } | { kind: 'error' } | { kind: 'ready'; view: PlayerView; chronicle: ChronicleEntry[] }

/**
 * The Journal tab (planche « Jouer », onglet Journal): what the whole
 * group knows, read from the server's player projection (`GET
 * /api/play/…/view`) — what the players were told, the scene they are
 * in, the clues found and the people met. Nothing here is written on the
 * phone. Below, the campaign's chronicle (session/write-recaps): one
 * entry per session the GM published, the latest first. `refreshKey`
 * moves when the live channel says the world, the story or the evening
 * changed.
 */
export function JournalTab({ campaignId, refreshKey }: { campaignId: string; refreshKey: number }) {
  const { t } = useTranslation()
  const [state, setState] = useState<State>({ kind: 'loading' })
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: State
    try {
      const [view, chronicle] = await Promise.all([fetchCampaignView(campaignId), fetchChronicle(campaignId)])
      next = { kind: 'ready', view, chronicle }
    } catch {
      next = { kind: 'error' }
    }
    if (request !== latest.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setState((s) => (next.kind === 'error' && s.kind === 'ready' ? s : next))
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  if (state.kind === 'loading') return <p role="status">{t('play.loading')}</p>
  if (state.kind === 'error') return <p role="alert">{t('play.error')}</p>
  const { view, chronicle } = state
  const nothingYet = !view.scene && view.clues.length === 0 && view.npcs.length === 0 && chronicle.length === 0

  return (
    <div className="flex flex-col gap-4">
      {view.playerHook && (
        <section className="flex flex-col gap-2">
          <h2 className="type-label">{t('play.journal.hook')}</h2>
          <p className="type-narration text-[18px] leading-snug text-chalk-soft">{view.playerHook}</p>
        </section>
      )}
      {view.scene && (
        <section className="flex flex-col gap-1.5">
          <h2 className="type-label">{t('play.journal.scene')}</h2>
          <span className="type-title text-[20px]">{view.scene.title}</span>
          {view.scene.place && <span className="text-caption text-mute-soft">{view.scene.place.name}</span>}
          {view.scene.readAloud && (
            <p className="type-narration text-[18px] leading-snug text-chalk-soft">{view.scene.readAloud}</p>
          )}
        </section>
      )}
      {view.clues.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="type-label">{t('play.journal.clues')}</h2>
          <ul className="flex flex-col gap-1.5">
            {view.clues.map((clue) => (
              <li key={clue} className="rounded-button bg-ivory px-3.5 py-3 text-body text-ink shadow-ivory-flat">
                {clue}
              </li>
            ))}
          </ul>
        </section>
      )}
      {view.npcs.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="type-label">{t('play.journal.npcs')}</h2>
          <ul className="flex flex-col gap-1.5">
            {view.npcs.map((npc) => (
              <li key={npc.id} className="flex flex-col rounded-button border border-line bg-well px-3 py-2.5">
                <b className="text-body">{npc.name}</b>
                {npc.title && <small className="text-caption text-mute-soft">{npc.title}</small>}
                {npc.appearance && <small className="text-caption text-mute">{npc.appearance}</small>}
              </li>
            ))}
          </ul>
        </section>
      )}
      {chronicle.length > 0 && (
        <section className="flex flex-col gap-2" aria-label={t('play.journal.chronicle')}>
          <h2 className="type-label">{t('play.journal.chronicle')}</h2>
          <ol className="flex flex-col gap-2 border-l border-line pl-3">
            {chronicle.map((e) => (
              <li key={e.number} className="flex flex-col gap-0.5">
                <span className="text-caption text-mute-soft">
                  {e.playedOn
                    ? t('play.journal.chronicleEntryOn', { number: e.number, date: shortDate(e.playedOn) })
                    : t('play.journal.chronicleEntry', { number: e.number })}
                </span>
                {e.title && <b className="type-title text-[18px]">{e.title}</b>}
                {e.text && <span className="text-body text-chalk-soft">{e.text}</span>}
              </li>
            ))}
          </ol>
          <p className="text-caption text-mute">{t('play.journal.chronicleNote')}</p>
        </section>
      )}
      {nothingYet && <p className="text-body text-mute">{t('play.journal.empty')}</p>}
    </div>
  )
}
