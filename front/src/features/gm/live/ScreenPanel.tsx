import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ApiError } from '@/lib/api'
import {
  fetchScreens,
  forgetScreen,
  openScreenWindow,
  pairScreen,
  setShows,
  type GmScreens,
  type Shows,
} from '@/lib/screens'
import { cn } from '@/lib/utils'
import { Btn, Panel, field } from './ui'

const SHOWS: (keyof Shows)[] = ['scene', 'map', 'party', 'moments']

/**
 * The shared screen, on the GM's live screen (session/pair-shared-screen,
 * planche « Lancer », moments 1 to 3): type the code a TV shows, or open
 * the TV window in this browser to share it on Discord; see which
 * screens follow the table and forget one; choose what they may show —
 * never more than every player may see.
 */
export function ScreenPanel({
  campaignId,
  online,
  refreshKey,
}: {
  campaignId: string
  /** Screens connected now (live presence). */
  online: string[]
  /** Moves when the live channel says the screens changed. */
  refreshKey: number
}) {
  const { t } = useTranslation()
  const [data, setData] = useState<GmScreens | null>(null)
  const [code, setCode] = useState('')
  const [error, setError] = useState<string | null>(null)
  const latest = useRef(0)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: GmScreens | null
    try {
      next = await fetchScreens(campaignId)
    } catch {
      next = null
    }
    if (request !== latest.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setData((d) => next ?? d)
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  async function act(call: () => Promise<GmScreens>): Promise<boolean> {
    setError(null)
    try {
      setData(await call())
      return true
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
      return false
    }
  }

  return (
    <Panel title={t('gmLive.screens.title')}>
      <form
        className="flex items-center gap-1.5"
        onSubmit={async (e) => {
          e.preventDefault()
          if (await act(() => pairScreen(campaignId, code))) setCode('')
        }}
      >
        <input
          className={cn(field, 'w-28 text-center font-bold tracking-[0.3em] uppercase')}
          value={code}
          maxLength={6}
          autoComplete="off"
          onChange={(e) => setCode(e.target.value.toUpperCase())}
          placeholder={t('gmLive.screens.codePlaceholder')}
          aria-label={t('gmLive.screens.code')}
        />
        <Btn type="submit" main disabled={code.trim().length < 4}>
          {t('gmLive.screens.pair')}
        </Btn>
        <Btn
          onClick={async () => {
            // Opened during the click, or a popup blocker eats it; it
            // loads `/tv` once this browser holds the window's token.
            const tv = window.open('', 'promptus-tv', 'popup,width=1280,height=720')
            if (await act(() => openScreenWindow(campaignId))) {
              if (tv) tv.location.href = '/tv'
            } else {
              tv?.close()
            }
          }}
        >
          {t('gmLive.screens.window')}
        </Btn>
      </form>
      <p className="text-caption text-mute-soft">{t('gmLive.screens.howTo')}</p>
      {error && (
        <p role="alert" className="text-caption text-stat-atk">
          {t(`gmLive.screens.errors.${error}`, { defaultValue: t('gmLive.screens.errors.UNEXPECTED') })}
        </p>
      )}
      {data && data.screens.length > 0 && (
        <ul className="flex flex-col gap-1.5">
          {data.screens.map((s) => {
            const on = online.includes(s.id)
            return (
              <li key={s.id} className="flex items-center justify-between gap-2 rounded-lg border border-line px-2.5 py-1.5">
                <span className="text-body font-bold">{t(`gmLive.screens.kind.${s.kind}`)}</span>
                <span className="text-caption text-mute-soft">{on ? t('gmLive.screens.online') : t('gmLive.screens.offline')}</span>
                <Btn onClick={() => void act(() => forgetScreen(campaignId, s.id))}>{t('gmLive.screens.forget')}</Btn>
              </li>
            )
          })}
        </ul>
      )}
      {data && (
        <fieldset className="flex flex-col gap-1">
          <legend className="type-label text-mute-soft">{t('gmLive.screens.shows')}</legend>
          {SHOWS.map((k) => (
            <label key={k} className="flex items-center gap-2 text-caption">
              <input
                type="checkbox"
                checked={data.shows[k]}
                onChange={(e) => void act(() => setShows(campaignId, { ...data.shows, [k]: e.target.checked }))}
              />
              {t(`gmLive.screens.show.${k}`)}
            </label>
          ))}
          <span className="text-caption text-mute">{t('gmLive.screens.never')}</span>
        </fieldset>
      )}
    </Panel>
  )
}
