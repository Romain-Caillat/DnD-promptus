import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { LiveScreen } from '@/lib/evening'
import { gmImageUrl, type MediaKind, type MediaList } from '@/lib/media'
import { Btn, Panel, field } from './ui'

const ASKABLE = ['scene', 'npc', 'tileset'] as const

/**
 * Images in the world's pixel art, reviewed by the GM
 * (media/draw-pixel-art-assets, maps/build-tileset-packs): ask for a
 * scene, an NPC or a material of the tileset, with a word of direction;
 * each drawing is a counted AI call and waits here until approved — only
 * then may the table see it, once its subject is theirs to see.
 */
export function MediaPanel({
  campaignId,
  screen,
  media,
  onAsk,
  onDecide,
}: {
  campaignId: string
  screen: LiveScreen
  media: MediaList | null
  onAsk: (kind: MediaKind, subject: string, direction: string) => Promise<void>
  onDecide: (asset: string, approve: boolean) => void
}) {
  const { t } = useTranslation()
  const subjects: Record<'scene' | 'npc' | 'tileset', { id: string; name: string }[]> = {
    scene: screen.nodes.map((n) => ({ id: n.id, name: n.title })),
    npc: (screen.scene?.npcs ?? []).map((n) => ({ id: n.id, name: n.name })),
    tileset: (media?.theme?.tilesets ?? []).flatMap((ts) =>
      Object.entries(ts.materials).map(([id, m]) => ({ id, name: `${m.name} (${ts.name})` })),
    ),
  }
  const [kind, setKind] = useState<'scene' | 'npc' | 'tileset'>('scene')
  const [subject, setSubject] = useState(screen.scene?.node.id ?? '')
  const [direction, setDirection] = useState('')
  const [busy, setBusy] = useState(false)
  const pending = (media?.assets ?? []).filter((a) => a.status === 'pending')
  const failed = (media?.assets ?? []).filter((a) => a.status === 'rejected' && a.error).slice(0, 2)
  const nameOf = (k: MediaKind, id: string) =>
    (subjects[k as keyof typeof subjects] ?? []).find((s) => s.id === id)?.name ?? id

  return (
    <Panel title={t('gmLive.media.title')}>
      <form
        className="flex flex-col gap-1.5"
        onSubmit={async (e) => {
          e.preventDefault()
          if (!subject) return
          setBusy(true)
          try {
            await onAsk(kind, subject, direction.trim())
            setDirection('')
          } finally {
            setBusy(false)
          }
        }}
      >
        <div className="flex gap-1.5">
          <select
            className={field}
            value={kind}
            onChange={(e) => {
              const k = e.target.value as typeof kind
              setKind(k)
              setSubject(subjects[k][0]?.id ?? '')
            }}
            aria-label={t('gmLive.media.kind')}
          >
            {ASKABLE.map((k) => (
              <option key={k} value={k}>
                {t(`gmLive.media.kinds.${k}`)}
              </option>
            ))}
          </select>
          <select
            className={`${field} min-w-0 flex-1`}
            value={subject}
            onChange={(e) => setSubject(e.target.value)}
            aria-label={t('gmLive.media.subject')}
          >
            <option value="">{t('gmLive.media.subject')}</option>
            {subjects[kind].map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </div>
        <input
          className={field}
          value={direction}
          onChange={(e) => setDirection(e.target.value)}
          maxLength={300}
          placeholder={t('gmLive.media.direction')}
          aria-label={t('gmLive.media.direction')}
        />
        <Btn type="submit" main disabled={busy || !subject || !screen.ai.configured}>
          {busy ? t('gmLive.media.drawing') : t('gmLive.media.ask')}
        </Btn>
      </form>
      {failed.map((a) => (
        <p key={a.id} className="text-caption text-stat-atk">
          {t('gmLive.media.failed', { subject: nameOf(a.kind, a.subject), error: a.error })}
        </p>
      ))}
      {pending.map((a) => (
        <figure key={a.id} className="flex flex-col gap-1.5 rounded-lg border border-line p-2">
          <img
            src={gmImageUrl(campaignId, a.id)}
            alt={nameOf(a.kind, a.subject)}
            className="w-full rounded-md [image-rendering:pixelated]"
          />
          <figcaption className="flex items-center justify-between gap-2 text-caption">
            <span>
              {t(`gmLive.media.kinds.${a.kind}`)} · {nameOf(a.kind, a.subject)}
              {a.direction && <span className="text-mute-soft"> · {a.direction}</span>}
            </span>
            <span className="flex gap-1.5">
              <Btn main onClick={() => onDecide(a.id, true)}>
                {t('gmLive.media.approve')}
              </Btn>
              <Btn onClick={() => onDecide(a.id, false)}>{t('gmLive.media.reject')}</Btn>
            </span>
          </figcaption>
        </figure>
      ))}
      <p className="text-caption text-mute-soft">
        {t('gmLive.media.approved', { count: (media?.assets ?? []).filter((a) => a.status === 'approved').length })}
      </p>
    </Panel>
  )
}
