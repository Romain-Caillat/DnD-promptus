import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useParams, useSearchParams } from 'react-router'
import { CardButton } from '@/components/game/CardButton'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { creatorPath } from '@/lib/creator'
import { DESKTOP_QUERY, useMediaQuery } from '@/lib/useMediaQuery'
import { cn } from '@/lib/utils'
import { fetchPlayerHome, type CharacterView, type PlayerHome } from '@/lib/play'
import type { PlayerTab } from '@/lib/between'
import { GameTab } from '@/features/evening/GameTab'
import { MapTab } from '@/features/map/MapTab'
import { Chronicle } from './between/Chronicle'
import { LevelUpPanel } from './between/LevelUpPanel'
import { CharacterTab } from './CharacterTab'
import { CharacterSummary } from './creator/ReviewStep'
import { FallenCard } from './FallenCard'
import { JournalTab } from './JournalTab'
import { RulesEntry } from './rules/RulesEntry'

type HomeState =
  | { kind: 'loading' }
  | { kind: 'not-joined' }
  | { kind: 'error' }
  | { kind: 'ready'; home: PlayerHome }

type Tab = PlayerTab

/**
 * `/partie/:campaignId` — a player's home in one campaign, phone first,
 * in tabs (planche « Jouer »): **Jeu** — the evening: lobby, scene,
 * music, cards, dice — **Carte** — the grid and the fights —
 * **Personnage** — the character being made and the way into the
 * creator, then, once validated, the sheet in play that the GM adjusts
 * live — and **Journal**, what the group knows. A spectator watches the
 * game, the map and the journal. Everything comes from the server's
 * player projection; the live channel says when to fetch it again.
 *
 * On a computer (player/play-on-desktop, planche « Jouer sur
 * ordinateur ») the same game unfolds side by side, without tabs: the
 * character on the left, the scene and the hand in the middle, the map,
 * the fight and the journal on the right; the keyboard plays the cards
 * and rolls the die. Each tab is mounted once either way.
 */
export function PlayerHomePage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { campaignId = '' } = useParams()
  const [params, setParams] = useSearchParams()
  const [state, setState] = useState<HomeState>({ kind: 'loading' })
  const [viewVersion, setViewVersion] = useState(0)
  const [eveningVersion, setEveningVersion] = useState(0)
  const [mapVersion, setMapVersion] = useState(0)
  const [fightTurn, setFightTurn] = useState(false)
  const desktop = useMediaQuery(DESKTOP_QUERY)
  const latest = useRef(0)
  const characterId = useRef<string | null>(null)

  const load = useCallback(async () => {
    const request = ++latest.current
    let next: HomeState
    try {
      const home = await fetchPlayerHome(campaignId)
      next = home ? { kind: 'ready', home } : { kind: 'not-joined' }
    } catch {
      next = { kind: 'error' }
    }
    if (request !== latest.current) return
    // A failed refetch keeps what is on screen; the next change retries.
    setState((s) => (next.kind === 'error' && s.kind === 'ready' ? s : next))
  }, [campaignId])

  useEffect(() => {
    void load()
  }, [load])

  useEffect(() => {
    // Once dead, the fallen character's changes (last words, choice) still concern me.
    characterId.current =
      state.kind === 'ready' ? (state.home.character?.id ?? state.home.fallen?.characterId ?? null) : null
  }, [state])

  useLiveChanges(
    campaignId,
    (topics) => {
      const mine = characterId.current
      if (mine && topics.includes(`character:${mine}`)) void load()
      const world = topics.includes('world') || topics.includes('story')
      if (world || topics.includes('session')) setViewVersion((v) => v + 1)
      if (world || topics.includes('session') || (mine && topics.includes(`character:${mine}`))) {
        setEveningVersion((v) => v + 1)
      }
      if (world || topics.includes('map') || topics.includes('fight')) setMapVersion((v) => v + 1)
    },
    'player',
  )

  if (state.kind !== 'ready') {
    return (
      <main className="surface-table mx-auto flex min-h-dvh max-w-md flex-col p-4 text-chalk">
        {state.kind === 'loading' && <p role="status">{t('play.loading')}</p>}
        {state.kind === 'not-joined' && <p role="alert">{t('play.notJoined')}</p>}
        {state.kind === 'error' && <p role="alert">{t('play.error')}</p>}
      </main>
    )
  }

  const { me, campaign, character } = state.home
  // My character died and no other sits in its place yet: still at the table.
  const fallen = me.role === 'player' && character === null ? (state.home.fallen ?? null) : null
  const seated = me.role === 'player' && character !== null
  const play = character?.play ?? null
  // The character comes first while it is being made; once in play, the
  // game; after a death, the death until the player chose what comes next.
  const tabs: Tab[] = fallen
    ? fallen.next
      ? ['jeu', 'carte', 'perso', 'journal']
      : ['perso', 'jeu', 'carte', 'journal']
    : !seated
      ? ['jeu', 'carte', 'journal']
      : play
        ? ['jeu', 'carte', 'perso', 'journal']
        : ['perso', 'jeu', 'carte', 'journal']
  const asked = params.get('onglet')
  const tab: Tab = tabs.find((x) => x === asked) ?? tabs[0]
  const bannerKey = fallen ? 'fallen' : !seated ? 'spectator' : play ? 'inPlay' : character.status
  const good = bannerKey === 'validated' || bannerKey === 'draft' || bannerKey === 'inPlay'

  function openTab(id: Tab) {
    setParams(id === tabs[0] ? {} : { onglet: id }, { replace: true })
  }

  function replaceCharacter(next: CharacterView) {
    setState((s) => (s.kind === 'ready' ? { ...s, home: { ...s.home, character: next } } : s))
  }

  const banner = (
    <p
      role="status"
      className={cn(
        'rounded-button px-3.5 py-2.5 text-body font-bold',
        good && 'bg-ivory text-ink shadow-ivory-flat',
        bannerKey === 'submitted' && 'border-[1.5px] border-dashed border-line-dashed bg-table text-chalk-soft',
        bannerKey === 'returned' && 'border-[1.5px] border-stat-atk bg-table text-chalk',
        bannerKey === 'spectator' && 'border border-line bg-surface text-chalk-soft',
        bannerKey === 'fallen' && 'border-[1.5px] border-stat-atk bg-table text-chalk',
      )}
    >
      {t(`play.banner.${bannerKey}`)}
    </p>
  )
  const title = (
    <>
      <span className="type-label">{t('play.hello', { nickname: me.nickname, gm: campaign.gmName })}</span>
      <h1 className="type-title text-heading">{campaign.title}</h1>
    </>
  )
  const characterPane = fallen ? (
    <FallenCard campaignId={campaignId} fallen={fallen} onChanged={(home) => setState({ kind: 'ready', home })} />
  ) : character ? (
    play ? (
      <>
        {play.upgradePoints > 0 && (
          <LevelUpPanel
            campaignId={campaignId}
            character={character}
            play={play}
            refreshKey={eveningVersion}
            onChanged={replaceCharacter}
          />
        )}
        <CharacterTab campaignId={campaignId} character={character} play={play} onChanged={replaceCharacter} />
      </>
    ) : (
      <CharacterCard character={character} gmName={campaign.gmName} onOpen={() => navigate(creatorPath(campaignId))} />
    )
  ) : null

  if (desktop) {
    return (
      <main className="surface-table flex min-h-dvh flex-col gap-4 p-5 text-chalk">
        <header className="flex flex-wrap items-end justify-between gap-4">
          <div className="flex flex-col gap-1">{title}</div>
          <div className="min-w-80">{banner}</div>
        </header>
        <div className="grid flex-1 grid-cols-[300px_minmax(0,1fr)_minmax(0,1.15fr)] items-start gap-5">
          <aside className="flex flex-col gap-3" aria-label={t(seated || fallen ? 'play.tabs.perso' : 'play.spectatorPane')}>
            {characterPane ?? <p className="text-body text-chalk-soft">{t('play.spectator')}</p>}
            <RulesEntry campaignId={campaignId} />
          </aside>
          <section className="flex flex-col gap-3" aria-label={t('play.tabs.jeu')}>
            <GameTab campaignId={campaignId} refreshKey={eveningVersion} seated={Boolean(play)} keyboard={!fightTurn} />
          </section>
          <div className="flex flex-col gap-5">
            <section className="flex flex-col gap-3" aria-label={t('play.tabs.carte')}>
              <MapTab campaignId={campaignId} refreshKey={mapVersion} keyboard onTurn={setFightTurn} />
            </section>
            <section className="flex flex-col gap-3" aria-label={t('play.tabs.journal')}>
              <h2 className="type-label">{t('play.tabs.journal')}</h2>
              <Chronicle campaignId={campaignId} refreshKey={eveningVersion} />
              <JournalTab campaignId={campaignId} refreshKey={viewVersion} />
            </section>
          </div>
        </div>
      </main>
    )
  }

  return (
    <main className="surface-table mx-auto flex min-h-dvh max-w-md flex-col text-chalk">
      <div className="mx-3 mt-4">{banner}</div>
      <header className="flex flex-col gap-1 px-4 pt-4">{title}</header>
      <section className="flex flex-1 flex-col gap-3 p-4" aria-label={t(`play.tabs.${tab}`)}>
        {tab === 'jeu' && (
          <GameTab campaignId={campaignId} refreshKey={eveningVersion} seated={Boolean(play)} onTab={openTab} />
        )}
        {tab === 'carte' && <MapTab campaignId={campaignId} refreshKey={mapVersion} />}
        {tab === 'journal' && (
          <>
            {!seated && !fallen && <p className="text-body text-chalk-soft">{t('play.spectator')}</p>}
            <Chronicle campaignId={campaignId} refreshKey={eveningVersion} />
            <JournalTab campaignId={campaignId} refreshKey={viewVersion} />
          </>
        )}
        {tab === 'perso' && characterPane}
        {(tab === 'perso' || tab === 'journal') && <RulesEntry campaignId={campaignId} />}
      </section>
      {tabs.length > 1 ? (
        <nav
          aria-label={t('play.tabs.label')}
          className="sticky bottom-0 grid auto-cols-fr grid-flow-col gap-1 border-t border-line bg-table px-3 pt-2 pb-[max(0.75rem,env(safe-area-inset-bottom))]"
        >
          {tabs.map((id) => (
            <button
              key={id}
              type="button"
              aria-current={tab === id ? 'page' : undefined}
              className={cn(
                'rounded-button py-2.5 text-caption font-bold tracking-[0.12em] uppercase',
                tab === id ? 'bg-ivory text-ink shadow-ivory-flat' : 'text-mute-soft',
              )}
              onClick={() => setParams(id === tabs[0] ? {} : { onglet: id }, { replace: true })}
            >
              {t(`play.tabs.${id}`)}
            </button>
          ))}
        </nav>
      ) : (
        <p className="px-4 pb-8 text-caption text-mute">{t('play.keep')}</p>
      )}
    </main>
  )
}

/** The character before it is in play: drawn once made, the GM's word, the way into the creator. */
function CharacterCard({
  character,
  gmName,
  onOpen,
}: {
  character: CharacterView
  gmName: string
  onOpen: () => void
}) {
  const { t } = useTranslation()
  return (
    <>
      <div className="surface-slab flex flex-col gap-2 p-3.5">
        <span className="type-label">{t('play.character')}</span>
        {character.sheet.look ? (
          <CharacterSummary
            sheet={character.sheet}
            look={character.sheet.look}
            subtitle={[character.peopleName, character.className, t('creator.review.level')].filter(Boolean).join(' · ')}
            stats={character.stats}
            over={0}
            gmName={gmName}
          />
        ) : (
          <span className="type-title text-[22px]">{character.sheet.name || t('play.unnamed')}</span>
        )}
        <p className="text-body text-chalk-soft">{t(`play.status.${character.status}`)}</p>
        {character.gmNote && (
          <div className="rounded-button bg-ivory px-3.5 py-3 text-body text-ink shadow-ivory-flat">
            <span className="type-label block text-ink-soft">{t('play.gmNote')}</span>
            {character.gmNote}
          </div>
        )}
        <CreatorButton character={character} onOpen={onOpen} />
      </div>
      <p className="text-caption text-mute">{t('play.keep')}</p>
    </>
  )
}

/** The way into the creator, while the sheet is the player's to edit. */
function CreatorButton({ character, onOpen }: { character: CharacterView; onOpen: () => void }) {
  const { t } = useTranslation()
  if (character.status === 'returned') {
    return <CardButton title={t('play.fix')} subtitle={t('play.fixHint')} onClick={onOpen} />
  }
  if (character.status !== 'draft') return null
  const started = Boolean(character.sheet.look || character.sheet.name)
  return (
    <CardButton
      title={t(started ? 'play.resume' : 'play.create')}
      subtitle={t('play.createHint')}
      onClick={onOpen}
    />
  )
}
