import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import type { LiveScreen } from '@/lib/evening'
import { Btn, Panel } from './ui'

/**
 * The launch of a session (gm/launch-session, planche « Lancer »,
 * moments 4 and 5). In the lobby: what is ready — the last recap
 * published, who is here with sound. Once launched: « Précédemment… »
 * whole for the GM, the sentences already on the phones lit, the next
 * one sent at the GM's pace, the GM's own recap beside it, then the
 * first scene — where the table stopped — sent everywhere at once.
 */
export function LaunchPanel({
  screen,
  onNext,
  onScene,
}: {
  screen: LiveScreen
  onNext: () => void
  onScene: (node: string) => void
}) {
  const { t } = useTranslation()
  const session = screen.session
  if (!session) return null

  if (session.status === 'lobby') {
    const players = screen.lobby.filter((s) => s.role === 'player')
    const here = players.filter((s) => s.here)
    const last = screen.lastEnded
    return (
      <Panel title={t('gmLive.launch.title')}>
        <ul className="flex flex-col gap-1.5 text-caption">
          <Check ok={!last || Boolean(last.publishedAt && last.previously)}>
            {!last
              ? t('gmLive.launch.firstSession')
              : last.publishedAt && last.previously
                ? t('gmLive.launch.recapPublished', { number: last.number })
                : t('gmLive.launch.recapDraft', { number: last.number })}
          </Check>
          <Check ok={players.length > 0 && here.length === players.length}>
            {players.length > 0 && here.length === players.length
              ? t('gmLive.launch.allHere')
              : t('gmLive.launch.here', {
                  here: here.length,
                  total: players.length,
                  sound: here.filter((s) => s.soundOk).length,
                })}
          </Check>
        </ul>
      </Panel>
    )
  }

  const launch = screen.launch
  if (session.status !== 'live' || !launch) return null
  const done = launch.shown >= launch.lines.length
  return (
    <Panel title={t('gmLive.launch.reading', { number: launch.number })}>
      <p className="text-caption text-mute-soft">{t('gmLive.launch.pace')}</p>
      <ol className="flex flex-col gap-1.5" aria-label={t('gmLive.launch.reading', { number: launch.number })}>
        {launch.lines.map((line, i) => (
          <li
            key={`${i}-${line}`}
            aria-current={i === launch.shown - 1 ? 'true' : undefined}
            className={cn(
              'type-narration text-[18px] leading-snug',
              i < launch.shown ? 'text-chalk' : 'text-mute',
            )}
          >
            {line}
          </li>
        ))}
      </ol>
      {screen.lastEnded?.recap && (
        <div className="rounded-button border border-line px-3 py-2 text-caption">
          <span className="type-label block">{t('gmLive.launch.forYou')}</span>
          <span className="whitespace-pre-line text-chalk-soft">{screen.lastEnded.recap}</span>
        </div>
      )}
      <div className="flex flex-wrap gap-1.5">
        <Btn main={!done} disabled={done} onClick={onNext}>
          {done ? t('gmLive.launch.allRead') : t('gmLive.launch.next')}
        </Btn>
        {launch.firstScene && (
          <Btn main={done} onClick={() => onScene(launch.firstScene!.node)}>
            {t('gmLive.launch.firstScene', { title: launch.firstScene.title })}
          </Btn>
        )}
      </div>
    </Panel>
  )
}

function Check({ ok, children }: { ok: boolean; children: ReactNode }) {
  const { t } = useTranslation()
  return (
    <li className={cn('flex items-start gap-2', ok ? 'text-chalk' : 'text-mute-soft')}>
      <span aria-hidden className="font-bold">
        {ok ? t('gmLive.launch.done') : t('gmLive.launch.todo')}
      </span>
      <span>{children}</span>
    </li>
  )
}
