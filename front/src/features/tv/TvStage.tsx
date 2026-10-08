import { useTranslation } from 'react-i18next'
import { FacetedDie, facesOf } from '@/components/game/FacetedDie'
import { GameCard } from '@/components/game/GameCard'
import { Hearts } from '@/components/game/Hearts'
import { MusicPlayer } from '@/features/evening/MusicPlayer'
import { eventLine } from '@/features/map/events'
import { MapCanvas } from '@/features/map/MapCanvas'
import { readGrid } from '@/features/map/render'
import { useImage } from '@/features/map/useImage'
import { useTileset } from '@/features/map/useTileset'
import { Sprite } from '@/features/sprites/Sprite'
import type { BoardView } from '@/lib/board'
import { imageOf, type MediaList } from '@/lib/media'
import { SCREEN_BACKDROP_URL, screenImageUrl, type ScreenSeat, type ScreenView } from '@/lib/screens'
import { cn } from '@/lib/utils'
import { feedLine, pickFocus, type Highlight } from './focus'

/** The map's box on the 1920 × 1080 stage, as on the TV board. */
const MAP_W = 1120
const MAP_H = 800

/**
 * The shared screen's one picture (tv/show-evening, planches « Écran
 * TV » et « TV · la soirée côté TV »): the campaign and the session at
 * the top, the music player (YouTube's stays visible), one focal point,
 * a one-line feed at the bottom, and a big moment over it all when one
 * happens. Built only from `ScreenView`, which holds nothing secret;
 * every word comes from the server's projection or `t()`.
 */
export function TvStage({ view, media, highlight }: { view: ScreenView; media: MediaList | null; highlight: Highlight | null }) {
  const { t } = useTranslation()
  const focus = pickFocus(view)
  const feed = focus === 'fight' ? fightFeed(view.board!, t) : feedLine(view)
  return (
    <div className="tv-screen">
      <header className="tv-top">
        <span className="type-title text-[32px]">{view.title}</span>
        {view.session && <span className="type-label text-[16px]">{t('tv.session', { number: view.session.number })}</span>}
      </header>
      {view.music && (
        <div className="tv-music">
          <MusicPlayer music={view.music} />
        </div>
      )}

      {focus === 'between' && <Between view={view} />}
      {focus === 'lobby' && <Lobby view={view} />}
      {focus === 'waiting' && <Lobby view={view} live />}
      {focus === 'previously' && <Previously text={view.previously!} />}
      {focus === 'scene' && <SceneFocus view={view} media={media} />}
      {(focus === 'map' || focus === 'fight') && <MapFocus view={view} media={media} fight={focus === 'fight'} />}

      {feed && focus !== 'between' && focus !== 'lobby' && (
        <p className="tv-feed" aria-live="polite">
          <span className="type-label text-[16px]">{t(focus === 'fight' ? 'tv.feed.fight' : 'tv.feed.table')}</span>
          <span className="truncate">{feed}</span>
        </p>
      )}
      {highlight && <HighlightOverlay key={highlight.id} highlight={highlight} />}
    </div>
  )
}

function fightFeed(board: BoardView, t: ReturnType<typeof useTranslation>['t']): string | null {
  const fight = board.fight
  if (!fight) return null
  const name = (id: string) => fight.order.find((f) => f.id === id)?.name ?? board.tokens.find((k) => k.id === id)?.name ?? id
  for (let i = fight.events.length - 1; i >= 0; i--) {
    const line = eventLine(fight.events[i], name, t)
    if (line) return line
  }
  return null
}

function Seat({ seat, size = 6 }: { seat: ScreenSeat; size?: number }) {
  return (
    <div className={cn('flex flex-col items-center gap-3', !seat.here && 'opacity-45')}>
      {seat.look ? <Sprite look={seat.look} scale={size} label={seat.name} /> : <span className="tv-noportrait" />}
      <span className="type-title text-[32px]">{seat.name}</span>
      <span className="text-[18px] text-mute-soft">{seat.nickname}</span>
    </div>
  )
}

function Lobby({ view, live = false }: { view: ScreenView; live?: boolean }) {
  const { t } = useTranslation()
  const here = view.party.filter((s) => s.here).length
  return (
    <div className="tv-center gap-8">
      <span className="type-label text-[18px]">{t('tv.tonight')}</span>
      <h1 className="type-title text-[112px] leading-none">{view.title}</h1>
      {view.session && <span className="type-label text-[18px]">{t('tv.session', { number: view.session.number })}</span>}
      {view.party.length > 0 && (
        <div className="mt-8 flex flex-wrap items-end justify-center gap-16">
          {view.party.map((s) => (
            <Seat key={`${s.nickname}-${s.name}`} seat={s} />
          ))}
        </div>
      )}
      <span className="tv-pulse mt-6 text-[26px] text-mute-soft">
        {live
          ? t('tv.waitingGm')
          : view.party.length > 0 && here === view.party.length
            ? t('tv.allHere')
            : t('tv.soon')}
      </span>
    </div>
  )
}

function Between({ view }: { view: ScreenView }) {
  const { t } = useTranslation()
  return (
    <div className="tv-center gap-10">
      <h1 className="type-title text-[96px] leading-none">{view.title}</h1>
      {view.tonight.length > 0 && (
        <>
          <span className="type-title text-[56px]">{t('tv.tonightRecap')}</span>
          <ul className="flex max-w-[1100px] flex-col gap-4 text-left text-[30px] leading-snug">
            {view.tonight.map((line, i) => (
              <li key={i}>— {line}</li>
            ))}
          </ul>
        </>
      )}
      {view.tonight.length === 0 && view.previously && <p className="type-narration max-w-[1300px] text-[40px]">{view.previously}</p>}
      {view.tonight.length === 0 && !view.previously && <span className="tv-pulse text-[26px] text-mute-soft">{t('tv.waitingGm')}</span>}
    </div>
  )
}

/** « Précédemment… », sentence by sentence. */
function Previously({ text }: { text: string }) {
  const { t } = useTranslation()
  const lines = text.match(/[^.!?…]+[.!?…]+|\S[^.!?…]*$/g) ?? [text]
  return (
    <div className="tv-center items-start gap-8 px-[200px] text-left">
      <span className="type-title text-[96px]">{t('tv.previously')}</span>
      {lines.map((l, i) => (
        <p key={i} className="tv-line type-narration m-0 text-[44px]" style={{ animationDelay: `${0.6 + i * 1.6}s` }}>
          {l.trim()}
        </p>
      ))}
    </div>
  )
}

function SceneFocus({ view, media }: { view: ScreenView; media: MediaList | null }) {
  const scene = view.scene!
  const art = imageOf(media, 'scene', scene.id)
  return (
    <>
      <div className="tv-art">{art && <img src={screenImageUrl(art.id)} alt="" className="size-full object-cover [image-rendering:pixelated]" />}</div>
      <div className="tv-lower">
        <h1 className="type-title text-[64px]">{scene.title}</h1>
        {scene.place && <span className="type-label text-[18px]">{scene.place.name}</span>}
        {scene.readAloud && <p className="type-narration m-0 text-[40px] text-chalk-soft">{scene.readAloud}</p>}
      </div>
    </>
  )
}

function MapFocus({ view, media, fight }: { view: ScreenView; media: MediaList | null; fight: boolean }) {
  const { t } = useTranslation()
  const board = view.board!
  const { tileset, atlases } = useTileset(board.map, media, screenImageUrl)
  const backdrop = useImage(board.map.backdrop?.image ? SCREEN_BACKDROP_URL : null)
  const grid = readGrid(board.map)
  const tile = Math.max(8, Math.floor(Math.min(MAP_W / Math.max(1, grid.width), MAP_H / Math.max(1, grid.height))))
  const order = board.fight?.live ? board.fight.order : []
  return (
    <>
      {fight && (
        <ol className="tv-track" aria-label={t('fight.order')}>
          {order.map((f) => (
            <li
              key={f.id}
              className={cn('tv-chip', f.id === board.fight?.active && 'tv-chip-now', !f.party && 'tv-chip-foe', f.standing !== 'in_fight' && 'opacity-35')}
            >
              {f.name}
            </li>
          ))}
        </ol>
      )}
      <div className="tv-map">
        <MapCanvas
          scene={{ map: board.map, tileset, atlases, backdrop, tokens: board.tokens, tile }}
          className="size-full overflow-hidden rounded-none border-0"
        />
      </div>
      <aside className="tv-party" aria-label={t('tv.party')}>
        <span className="type-label text-[16px]">{t('tv.party')}</span>
        {view.party.map((s) => {
          const fighter = board.fight?.order.find((f) => f.party && f.name === s.name)
          const hp = fighter?.hitPoints ?? s.hitPoints
          const max = fighter?.maxHitPoints ?? s.maxHitPoints
          return (
            <div key={`${s.nickname}-${s.name}`} className={cn('tv-pc', fighter && fighter.id === board.fight?.active && 'tv-pc-now')}>
              {s.look ? <Sprite look={s.look} scale={2} /> : <span className="tv-noportrait tv-noportrait-sm" />}
              <div className="flex min-w-0 flex-1 flex-col gap-2">
                <div className="flex items-baseline justify-between gap-2">
                  <span className="type-title truncate text-[24px]">{s.name}</span>
                  <span className="text-[18px] text-mute-soft">{s.nickname}</span>
                </div>
                {hp !== null && max !== null && (
                  <div className="flex items-center gap-3">
                    <Hearts hp={hp} max={max} count={8} px={3} />
                    <span className="text-[24px] font-bold text-stat-hp tabular-nums">
                      {hp} / {max}
                    </span>
                  </div>
                )}
              </div>
            </div>
          )
        })}
      </aside>
    </>
  )
}

/** « + 4 », « − 1 »: the modifier as the dice read it. */
function signed(n: number): string {
  return n < 0 ? `− ${-n}` : `+ ${n}`
}

function HighlightOverlay({ highlight }: { highlight: Highlight }) {
  const { t } = useTranslation()
  if (highlight.kind === 'roll') {
    const r = highlight.roll
    return (
      <div className="tv-moment" role="status">
        <span className="type-label text-[20px]">{r.character}</span>
        <span className="type-title text-[64px]">{r.ability}</span>
        <span className="text-[32px] text-chalk-soft">{t('tv.roll.against', { difficulty: r.difficulty })}</span>
        <FacetedDie faces={facesOf(r.roll.die)} value={r.roll.natural} className="size-[300px] [&_span]:text-[96px]" />
        <span className="text-[64px] font-bold tabular-nums">
          {t('tv.roll.total', { natural: r.roll.natural, total: r.roll.total, modifier: signed(r.roll.total - r.roll.natural) })}
        </span>
        {r.outcome && <span className="tv-slam text-[64px]">{r.outcome}</span>}
      </div>
    )
  }
  if (highlight.kind === 'hit') {
    return (
      <div className="tv-moment" role="status">
        <span className="type-title text-[64px]">{highlight.target}</span>
        <span className="tv-hit text-[180px] font-bold leading-none tabular-nums text-destructive">− {highlight.amount}</span>
        {highlight.down && <span className="tv-slam tv-slam-now text-[56px]">{t('tv.moment.down')}</span>}
      </div>
    )
  }
  if (highlight.kind === 'clue') {
    return (
      <div className="tv-moment tv-rays" role="status">
        <span className="type-label text-[20px]">{t('tv.moment.clue')}</span>
        <div className="tv-flip">
          <GameCard kind="clue" title={t('tv.moment.clueTitle')} text={highlight.text} width={420} deal={false} />
        </div>
      </div>
    )
  }
  return (
    <div className="tv-moment tv-rays" role="status">
      <span className="type-label text-[20px]">{t(`tv.moment.${highlight.kind}`)}</span>
      <span className="type-title max-w-[1400px] text-[72px] leading-tight">{highlight.text}</span>
    </div>
  )
}
