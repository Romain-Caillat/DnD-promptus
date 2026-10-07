import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { AffinityGauge } from '@/components/game/AffinityGauge'
import type { PlayerView } from '@/lib/campaigns'
import { fetchCampaignView } from '@/lib/play'

type State = { kind: 'loading' } | { kind: 'error' } | { kind: 'ready'; view: PlayerView }

/**
 * The Journal tab (planche « Jouer », onglet Journal): what the whole
 * group knows, read from the server's player projection (`GET
 * /api/play/…/view`) — what the players were told, the scene they are
 * in, the clues found, the people and factions met and the campaign's
 * goals. Nothing here is written on the
 * phone. `refreshKey` moves when the live channel says the world or the
 * story changed.
 */
export function JournalTab({ campaignId, refreshKey }: { campaignId: string; refreshKey: number }) {
  const { t } = useTranslation()
  const [state, setState] = useState<State>({ kind: 'loading' })
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: State
    try {
      next = { kind: 'ready', view: await fetchCampaignView(campaignId) }
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
  const { view } = state
  const nothingYet = !view.scene && view.clues.length === 0 && view.npcs.length === 0

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
      {view.factions.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="type-label">{t('play.journal.factions')}</h2>
          <ul className="flex flex-col gap-1.5">
            {view.factions.map((f) => (
              <li key={f.id} className="flex flex-col gap-1.5 rounded-button border border-line bg-well px-3 py-2.5">
                <b className="text-body">{f.name}</b>
                <AffinityGauge value={f.affinity} min={f.min} max={f.max} label={t('play.journal.affinity', { name: f.name })} />
              </li>
            ))}
          </ul>
        </section>
      )}
      {view.goals.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="type-label">{t('play.journal.goals')}</h2>
          <ul className="flex flex-col gap-1.5">
            {view.goals.map((g) => (
              <li key={g.id} className="flex items-center justify-between gap-2 rounded-button border border-line px-3 py-2.5">
                <span className={g.done ? 'text-body text-mute line-through' : 'text-body'}>{g.title}</span>
                <small className="text-caption font-bold">
                  {g.done ? t('play.journal.goalDone') : t('play.journal.goalOpen')}
                </small>
              </li>
            ))}
          </ul>
        </section>
      )}
      {nothingYet && <p className="text-body text-mute">{t('play.journal.empty')}</p>}
    </div>
  )
}
