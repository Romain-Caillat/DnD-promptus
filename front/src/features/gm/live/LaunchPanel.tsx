import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { LiveScreen } from '@/lib/evening'
import type { Screen } from '@/lib/tv'
import { cn } from '@/lib/utils'
import { Btn, Panel, field } from './ui'

/** What the shared screen may show, against a player's phone (planche « Lancer », moment 3). */
type Seen = 'map' | 'party' | 'moments' | 'sheet' | 'notes' | 'copilot'
const SEES: [Seen, boolean, boolean][] = [
  ['map', true, true],
  ['party', true, true],
  ['moments', true, true],
  ['sheet', false, true],
  ['notes', false, false],
  ['copilot', false, false],
]

const TIME: Intl.DateTimeFormatOptions = { hour: '2-digit', minute: '2-digit' }

/**
 * The launch (gm/launch-session, session/pair-shared-screen, planche
 * « Lancer »): pair a TV by its code or open a window to share on
 * Discord, forget a screen, what the screen may show, what is left
 * before starting, then « Précédemment… » read line by line — the
 * screen and the phones follow.
 */
export function LaunchPanel({
  screen,
  screens,
  onPair,
  onWindow,
  onForget,
  onReading,
}: {
  screen: LiveScreen
  screens: Screen[]
  onPair: (code: string) => Promise<boolean>
  onWindow: () => void
  onForget: (screen: string) => void
  onReading: (line: number | null) => void
}) {
  const { t, i18n } = useTranslation()
  const [code, setCode] = useState('')
  const session = screen.session
  const lines = screen.readingLines
  const shown = session?.readingLine ?? null
  const players = screen.lobby.filter((s) => s.role === 'player')
  const missing = players.filter((s) => !s.here)
  const checks: [string, boolean][] = [
    [t('gmLive.launch.checks.previously'), lines.length > 0],
    [screens.length > 0 ? t('gmLive.launch.checks.screen', { name: screens[0].name }) : t('gmLive.launch.checks.noScreen'), screens.length > 0],
    [
      missing.length === 0
        ? t('gmLive.launch.checks.allHere', { count: players.length })
        : t('gmLive.launch.checks.missing', { names: missing.map((s) => s.nickname).join(', ') }),
      missing.length === 0,
    ],
  ]

  return (
    <Panel title={t('gmLive.launch.title')}>
      <span className="type-label">{t('gmLive.launch.shared')}</span>
      <div className="grid gap-2 sm:grid-cols-2">
        <form
          className="flex flex-col gap-2 rounded-lg border border-line p-2.5"
          onSubmit={(e) => {
            e.preventDefault()
            void onPair(code).then((ok) => ok && setCode(''))
          }}
        >
          <b className="text-body">{t('gmLive.launch.tv')}</b>
          <span className="text-caption text-mute-soft">{t('gmLive.launch.tvHint')}</span>
          <div className="flex gap-2">
            <input
              aria-label={t('gmLive.launch.code')}
              value={code}
              onChange={(e) => setCode(e.target.value.toUpperCase())}
              maxLength={4}
              autoCapitalize="characters"
              className={cn(field, 'w-24 font-mono tracking-[.3em]')}
            />
            <Btn type="submit" disabled={code.trim().length !== 4}>
              {t('gmLive.launch.pair')}
            </Btn>
          </div>
        </form>
        <div className="flex flex-col gap-2 rounded-lg border border-line p-2.5">
          <b className="text-body">{t('gmLive.launch.window')}</b>
          <span className="text-caption text-mute-soft">{t('gmLive.launch.windowHint')}</span>
          <Btn className="self-start" onClick={onWindow}>
            {t('gmLive.launch.openWindow')}
          </Btn>
        </div>
      </div>
      {screens.length > 0 ? (
        <ul className="flex flex-col gap-1.5" aria-label={t('gmLive.launch.screens')}>
          {screens.map((s) => (
            <li key={s.id} className="flex items-center justify-between gap-2 rounded-lg border border-line px-2.5 py-2">
              <span className="text-body">
                <b>{s.name}</b>{' '}
                <span className="text-caption text-mute-soft">
                  {t('gmLive.launch.pairedAt', {
                    time: new Date(s.pairedAt).toLocaleTimeString(i18n.language, TIME),
                  })}
                </span>
              </span>
              <Btn onClick={() => onForget(s.id)}>{t('gmLive.launch.forget')}</Btn>
            </li>
          ))}
        </ul>
      ) : (
        <p className="text-caption text-mute-soft">{t('gmLive.launch.optional')}</p>
      )}

      <details>
        <summary className="type-label cursor-pointer">{t('gmLive.launch.sees.title')}</summary>
        <table className="mt-2 w-full text-caption">
          <thead>
            <tr className="text-mute-soft">
              <th className="text-left font-normal">{t('gmLive.launch.sees.content')}</th>
              <th className="font-normal">{t('gmLive.launch.sees.tv')}</th>
              <th className="font-normal">{t('gmLive.launch.sees.players')}</th>
            </tr>
          </thead>
          <tbody>
            {SEES.map(([key, tv, players]) => (
              <tr key={key}>
                <td className="py-1">{t(`gmLive.launch.sees.rows.${key}`)}</td>
                <td className={cn('text-center', tv ? 'text-chalk' : 'text-mute')}>{tv ? t('gmLive.launch.yes') : t('gmLive.launch.no')}</td>
                <td className={cn('text-center', players ? 'text-chalk' : 'text-mute')}>
                  {players ? t('gmLive.launch.yes') : t('gmLive.launch.no')}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="mt-1 text-caption text-mute-soft">{t('gmLive.launch.sees.note')}</p>
      </details>

      <span className="type-label">{t('gmLive.launch.before')}</span>
      <ul className="flex flex-col gap-1" aria-label={t('gmLive.launch.before')}>
        {checks.map(([label, ok]) => (
          <li key={label} className={cn('text-caption', ok ? 'text-chalk' : 'text-mute-soft')}>
            {ok ? t('gmLive.launch.done') : t('gmLive.launch.pending')} {label}
          </li>
        ))}
      </ul>

      {session && lines.length > 0 && (
        <section className="flex flex-col gap-2" aria-label={t('evening.previously')}>
          <span className="type-label">{t('evening.previously')}</span>
          <ol className="flex flex-col gap-1">
            {lines.map((line, i) => (
              <li key={i} className={cn('type-narration text-[16px]', shown !== null && i < shown ? 'text-chalk' : 'text-mute')}>
                {line}
              </li>
            ))}
          </ol>
          <div className="flex flex-wrap gap-2">
            {shown === null ? (
              <Btn main onClick={() => onReading(1)}>
                {t('gmLive.launch.read')}
              </Btn>
            ) : (
              <>
                {shown < lines.length && (
                  <Btn main onClick={() => onReading(shown + 1)}>
                    {t('gmLive.launch.nextLine')}
                  </Btn>
                )}
                <Btn onClick={() => onReading(null)}>{t('gmLive.launch.stopReading')}</Btn>
              </>
            )}
          </div>
        </section>
      )}
    </Panel>
  )
}
