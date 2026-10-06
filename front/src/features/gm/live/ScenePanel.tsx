import { useTranslation } from 'react-i18next'
import { ThreatClock } from '@/components/game/ThreatClock'
import type { LiveScreen, Reveal } from '@/lib/evening'
import { cn } from '@/lib/utils'
import { Btn, Panel } from './ui'

/**
 * The scene the table is in, whole (gm/run-live-screen): the text to read
 * aloud, the GM's notes and key points, its clues and NPCs to reveal in
 * one tap, its music by mood, its exits — each one a tap that takes the
 * table there — and the fronts' clocks. Before the first scene, the
 * scenes to open with.
 */
export function ScenePanel({
  screen,
  onReveal,
  onTrack,
}: {
  screen: LiveScreen
  onReveal: (r: Reveal) => void
  onTrack: (track: number | null) => void
}) {
  const { t } = useTranslation()
  const live = screen.session?.status === 'live'
  const scene = screen.scene
  if (!scene) {
    return (
      <Panel title={t('gmLive.scene.choose')}>
        <ul className="flex flex-col gap-1">
          {screen.nodes.map((n) => (
            <li key={n.id} className="flex items-center justify-between gap-2">
              <span className={cn('text-body', n.id === screen.startNode && 'font-bold')}>{n.title}</span>
              <Btn main={n.id === screen.startNode} disabled={!live} onClick={() => onReveal({ kind: 'scene', node: n.id })}>
                {t('gmLive.scene.enter')}
              </Btn>
            </li>
          ))}
        </ul>
        {!live && <p className="text-caption text-mute">{t('gmLive.scene.notLive')}</p>}
      </Panel>
    )
  }
  const node = scene.node
  const tracks = node.ambience?.music ?? []
  const playing = screen.session?.music
  return (
    <Panel
      title={t('gmLive.scene.title')}
      actions={
        <Btn disabled={!live} onClick={() => onReveal({ kind: 'resolve', node: node.id })}>
          {t('gmLive.scene.resolve')}
        </Btn>
      }
    >
      <h3 className="type-title text-[22px]">{node.title}</h3>
      {node.read_aloud && (
        <blockquote className="type-narration border-l-2 border-ivory pl-3 text-[18px] leading-snug">{node.read_aloud}</blockquote>
      )}
      {node.flow && <p className="text-body text-chalk-soft">{node.flow}</p>}
      {node.key_points && node.key_points.length > 0 && (
        <ul className="list-disc pl-5 text-body text-chalk-soft">
          {node.key_points.map((k) => (
            <li key={k}>{k}</li>
          ))}
        </ul>
      )}
      {node.gm_notes && <p className="rounded-lg border border-dashed border-line-dashed p-2 text-caption">{node.gm_notes}</p>}

      {tracks.length > 0 && (
        <div className="flex flex-col gap-1">
          <span className="type-label">{t('gmLive.scene.music')}</span>
          <div className="flex flex-wrap gap-1.5">
            {tracks.map((tr, i) => (
              <Btn
                key={i}
                main={playing?.title === tr.title}
                disabled={!live || !tr.url}
                title={tr.url ? tr.title : t('gmLive.scene.noLink', { search: tr.search ?? tr.title })}
                onClick={() => onTrack(i)}
              >
                {t(`evening.music.mood.${tr.mood}`)} · {tr.title}
              </Btn>
            ))}
            {playing && (
              <Btn disabled={!live} onClick={() => onTrack(null)}>
                {t('gmLive.scene.stop')}
              </Btn>
            )}
          </div>
        </div>
      )}

      {scene.clues.length > 0 && (
        <div className="flex flex-col gap-1">
          <span className="type-label">{t('gmLive.scene.clues')}</span>
          {scene.clues.map((c) => (
            <div key={c.id} className="flex items-start justify-between gap-2">
              <span className={cn('text-body', c.found && 'text-mute line-through')}>
                {c.text}
                <span className="block text-caption text-mute-soft">{c.discovery}</span>
              </span>
              {!c.found && (
                <Btn disabled={!live} onClick={() => onReveal({ kind: 'clue', clue: c.id })}>
                  {t('gmLive.scene.reveal')}
                </Btn>
              )}
            </div>
          ))}
        </div>
      )}

      {scene.npcs.length > 0 && (
        <div className="flex flex-col gap-1">
          <span className="type-label">{t('gmLive.scene.npcs')}</span>
          {scene.npcs.map((n) => (
            <details key={n.id} className="rounded-lg border border-line px-2 py-1.5">
              <summary className="flex cursor-pointer items-center justify-between gap-2 text-body font-bold">
                {n.name}
                {!n.met && (
                  <Btn disabled={!live} onClick={() => onReveal({ kind: 'npc', npc: n.id })}>
                    {t('gmLive.scene.meet')}
                  </Btn>
                )}
              </summary>
              <p className="text-caption">{n.roleplay}</p>
              <p className="text-caption text-mute-soft">{t('gmLive.scene.wants', { wants: n.wants })}</p>
              <p className="text-caption text-mute-soft">{t('gmLive.scene.hides', { hides: n.hides })}</p>
            </details>
          ))}
        </div>
      )}

      {scene.exits.length > 0 && (
        <div className="flex flex-col gap-1">
          <span className="type-label">{t('gmLive.scene.exits')}</span>
          {scene.exits.map((e) => (
            <div key={e.to} className="flex items-center justify-between gap-2">
              <span className="text-body">
                {e.title}
                {e.label && <span className="text-caption text-mute-soft"> · {e.label}</span>}
              </span>
              <Btn disabled={!live} onClick={() => onReveal({ kind: 'scene', node: e.to })}>
                {t('gmLive.scene.go')}
              </Btn>
            </div>
          ))}
        </div>
      )}

      <details>
        <summary className="cursor-pointer text-caption text-mute-soft">{t('gmLive.scene.other')}</summary>
        <ul className="mt-1 flex flex-col gap-1">
          {screen.nodes
            .filter((n) => !n.current)
            .map((n) => (
              <li key={n.id} className="flex items-center justify-between gap-2 text-caption">
                <span className={cn(n.visited && 'text-mute')}>{n.title}</span>
                <Btn disabled={!live} onClick={() => onReveal({ kind: 'scene', node: n.id })}>
                  {t('gmLive.scene.go')}
                </Btn>
              </li>
            ))}
        </ul>
      </details>

      {screen.fronts.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <span className="type-label">{t('gmLive.scene.fronts')}</span>
          {screen.fronts.map((f) => (
            <div key={f.id} className="flex items-center gap-2">
              <ThreatClock parts={f.steps.length} filled={f.progress} size={48} next={false} showCount={false} name={f.name} />
              <div className="flex min-w-0 flex-1 flex-col">
                <span className="text-body font-bold">{f.name}</span>
                <span className="truncate text-caption text-mute-soft">
                  {f.steps[Math.max(0, f.progress - 1)] ?? f.goal}
                </span>
              </div>
              <Btn disabled={!live || f.progress === 0} onClick={() => onReveal({ kind: 'front', front: f.id, delta: -1 })}>
                {t('gmLive.scene.back')}
              </Btn>
              <Btn disabled={!live} onClick={() => onReveal({ kind: 'front', front: f.id, delta: 1 })}>
                {t('gmLive.scene.advance')}
              </Btn>
            </div>
          ))}
        </div>
      )}
    </Panel>
  )
}
