import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { musicOffset, youtubeId, type Music } from '@/lib/evening'

/** How far a phone may drift from the table before it seeks back (an ad, a stall). */
const DRIFT_S = 4
const CHECK_MS = 10_000

interface YtPlayer {
  seekTo(seconds: number, allowSeekAhead: boolean): void
  getCurrentTime(): number
  mute(): void
  unMute(): void
  playVideo(): void
  destroy(): void
}

interface YtNamespace {
  Player: new (
    el: HTMLElement,
    opts: {
      videoId: string
      playerVars: Record<string, number>
      events: { onReady: () => void }
    },
  ) => YtPlayer
}

declare global {
  interface Window {
    YT?: YtNamespace
    onYouTubeIframeAPIReady?: () => void
  }
}

let apiLoading: Promise<YtNamespace> | null = null

/** The YouTube IFrame API, loaded once per page. */
function youtubeApi(): Promise<YtNamespace> {
  if (window.YT?.Player) return Promise.resolve(window.YT)
  apiLoading ??= new Promise((resolve) => {
    const previous = window.onYouTubeIframeAPIReady
    window.onYouTubeIframeAPIReady = () => {
      previous?.()
      if (window.YT) resolve(window.YT)
    }
    const script = document.createElement('script')
    script.src = 'https://www.youtube.com/iframe_api'
    document.head.appendChild(script)
  })
  return apiLoading
}

/**
 * The visible mini-player (media/play-youtube-music, YouTube requires
 * one): the GM's track for everyone, started at the minute the table is
 * at (`startedAt` on the server), muted until the player taps « Activer
 * le son » — phones refuse sound without a gesture. Every few seconds it
 * checks how far it drifted (an ad, a stall) and seeks back to the
 * table. A new track from the GM replaces it on every phone.
 */
export function MusicPlayer({ music }: { music: Music }) {
  const { t } = useTranslation()
  const box = useRef<HTMLDivElement>(null)
  const player = useRef<YtPlayer | null>(null)
  const [sound, setSound] = useState(false)
  const id = youtubeId(music.url)

  useEffect(() => {
    if (!id || !box.current) return
    let gone = false
    const host = document.createElement('div')
    box.current.replaceChildren(host)
    let timer = 0
    void youtubeApi().then((YT) => {
      if (gone) return
      player.current = new YT.Player(host, {
        videoId: id,
        playerVars: { autoplay: 1, playsinline: 1, controls: 0, start: musicOffset(music), loop: 1 },
        events: {
          onReady: () => {
            const p = player.current
            if (!p) return
            p.mute()
            p.seekTo(musicOffset(music), true)
            p.playVideo()
            timer = window.setInterval(() => {
              const drift = Math.abs(p.getCurrentTime() - musicOffset(music))
              if (drift > DRIFT_S) p.seekTo(musicOffset(music), true)
            }, CHECK_MS)
          },
        },
      })
    })
    return () => {
      gone = true
      window.clearInterval(timer)
      player.current?.destroy()
      player.current = null
      setSound(false)
    }
    // The track and its start are the whole identity of what plays.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, music.startedAt])

  if (!id) return null
  return (
    <section className="surface-slab flex items-center gap-3 p-2.5" aria-label={t('evening.music.label')}>
      <div ref={box} className="aspect-video w-28 shrink-0 overflow-hidden rounded-md bg-black [&_iframe]:size-full" />
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="type-label">{t(`evening.music.mood.${music.mood}`)}</span>
        <span className="truncate text-body font-bold">{music.title}</span>
        <button
          type="button"
          className="self-start rounded-button bg-ivory px-3 py-1.5 text-caption font-bold text-ink shadow-ivory-flat"
          onClick={() => {
            const p = player.current
            if (!p) return
            if (sound) p.mute()
            else {
              p.unMute()
              p.seekTo(musicOffset(music), true)
              p.playVideo()
            }
            setSound(!sound)
          }}
        >
          {t(sound ? 'evening.music.mute' : 'evening.music.unmute')}
        </button>
      </div>
    </section>
  )
}
