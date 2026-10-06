import { useTranslation } from 'react-i18next'
import type { LiveScreen } from '@/lib/evening'
import { cn } from '@/lib/utils'
import { Btn, Panel } from './ui'

/**
 * The table (gm/balance-spotlight): who is here, sound checked, online;
 * then for each player how long since their last moment, their pending
 * requests and the secret hooks to play — the longest idle first,
 * flagged past the alert. « Donner la main » records a moment.
 */
export function TablePanel({ screen, onSpotlight }: { screen: LiveScreen; onSpotlight: (player: string) => void }) {
  const { t } = useTranslation()
  const live = screen.session?.status === 'live'
  return (
    <Panel title={t('gmLive.table.title')}>
      <ul className="flex flex-col gap-1.5">
        {screen.lobby.map((s) => {
          const spot = screen.spotlight.find((x) => x.playerId === s.playerId)
          return (
            <li
              key={s.playerId}
              className={cn('flex flex-col gap-1 rounded-lg border px-2.5 py-2', spot?.alert ? 'border-stat-atk' : 'border-line')}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="text-body font-bold">
                  {s.characterName ?? s.nickname}
                  {s.characterName && <span className="font-normal text-mute-soft"> · {s.nickname}</span>}
                </span>
                <span className="text-caption text-mute-soft">
                  {[
                    s.online ? t('gmLive.table.online') : t('gmLive.table.offline'),
                    s.here ? t('gmLive.table.here') : null,
                    s.here && s.soundOk ? t('gmLive.table.sound') : null,
                    s.role === 'spectator' ? t('gmLive.table.spectator') : null,
                  ]
                    .filter(Boolean)
                    .join(' · ')}
                </span>
              </div>
              {spot && (
                <div className="flex items-center justify-between gap-2">
                  <span className={cn('text-caption', spot.alert ? 'text-stat-atk' : 'text-mute-soft')}>
                    {t('gmLive.table.idle', { count: spot.idleMinutes })}
                    {spot.pendingRequests > 0 && ` · ${t('gmLive.table.pending', { count: spot.pendingRequests })}`}
                  </span>
                  {live && (
                    <Btn main={spot.alert} onClick={() => onSpotlight(s.playerId)}>
                      {t('gmLive.table.give')}
                    </Btn>
                  )}
                </div>
              )}
              {spot && spot.hooks.length > 0 && (
                <p className="text-caption text-chalk-soft">{t('gmLive.table.hooks', { hooks: spot.hooks.map((h) => h.title).join(' · ') })}</p>
              )}
            </li>
          )
        })}
      </ul>
    </Panel>
  )
}
