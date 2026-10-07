import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useParams } from 'react-router'
import { Button } from '@/components/ui/button'
import { CardButton } from '@/components/game/CardButton'
import { LiveIndicator } from '@/features/live/LiveIndicator'
import { useLiveChanges } from '@/features/live/useLiveChanges'
import { Sprite } from '@/features/sprites/Sprite'
import { ApiError } from '@/lib/api'
import {
  fetchInvite,
  fetchPlayerPreview,
  listSeats,
  removeSeat,
  type InviteStatus,
  type PlayerPreview,
  type Seat,
} from '@/lib/table'
import { cn } from '@/lib/utils'
import { CharacterReview } from './CharacterReview'
import { PlaySheets } from './PlaySheets'
import { SecretHooks } from './SecretHooks'
import { TableInvite } from './TableInvite'

const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long', timeStyle: 'short' })

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; preview: PlayerPreview; invite: InviteStatus | null }

/** What the centre of the page shows. */
type View =
  | { kind: 'invite' }
  | { kind: 'review'; characterId: string }
  | { kind: 'hooks' }
  | { kind: 'sheets' }

/** How a seat's status reads on the table (board « Inviter »). */
type SeatTone = 'ok' | 'todo' | 'wait' | 'off'

/**
 * `/campagnes/:campaignId/table` — the GM's table (board « Inviter »): the
 * invitation, the table filling up live, the review of each sheet, the
 * secret hooks drawn from the backstories, and the characters in play
 * with their one-gesture adjustments. The seat list and the sheets in
 * play follow the live channel's `table` topic; the open review follows
 * its character's topic.
 */
export function GmTablePage() {
  const { t } = useTranslation()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [seats, setSeats] = useState<Seat[]>([])
  const [view, setView] = useState<View>({ kind: 'invite' })
  const [reviewVersion, setReviewVersion] = useState(0)
  const [tableVersion, setTableVersion] = useState(0)
  const [actionError, setActionError] = useState(false)
  const [confirming, setConfirming] = useState<string | null>(null)
  const latestSeats = useRef(0)

  useEffect(() => {
    let live = true
    Promise.all([fetchPlayerPreview(campaignId), fetchInvite(campaignId)]).then(
      ([preview, invite]) => {
        if (live) setState({ kind: 'ready', preview, invite })
      },
      (err: unknown) => {
        if (!live) return
        if (err instanceof ApiError && err.status === 401) setState({ kind: 'signed-out' })
        else if (err instanceof ApiError && err.status === 404) setState({ kind: 'not-found' })
        else setState({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [campaignId])

  const loadSeats = useCallback(async () => {
    const request = ++latestSeats.current
    let list: Seat[] | null
    try {
      list = await listSeats(campaignId)
    } catch {
      // Kept as it was; the next change tries again.
      list = null
    }
    if (list && request === latestSeats.current) setSeats(list)
  }, [campaignId])

  useEffect(() => {
    void loadSeats()
  }, [loadSeats])

  const reviewing = view.kind === 'review' ? view.characterId : null
  const live = useLiveChanges(campaignId, (topics) => {
    if (topics.includes('table')) {
      void loadSeats()
      setTableVersion((v) => v + 1)
    }
    if (reviewing && topics.includes(`character:${reviewing}`)) setReviewVersion((v) => v + 1)
  })

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />

  async function remove(seat: Seat) {
    setActionError(false)
    try {
      await removeSeat(campaignId, seat.id)
      setSeats((list) => list.filter((s) => s.id !== seat.id))
      if (seat.character && reviewing === seat.character.id) setView({ kind: 'invite' })
    } catch {
      setActionError(true)
    } finally {
      setConfirming(null)
    }
  }

  if (state.kind !== 'ready') {
    return (
      <main className="surface-table flex min-h-dvh flex-col p-6 text-chalk">
        {state.kind === 'loading' && <p role="status">{t('gm.table.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('gm.table.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('gm.table.error')}</p>}
      </main>
    )
  }

  const characters = seats.filter((s) => s.character)
  const validated = characters.filter((s) => s.character?.status === 'validated').length

  return (
    <main className="surface-table flex min-h-dvh flex-col text-chalk">
      <header className="flex flex-wrap items-center gap-4 border-b border-line px-5 py-3 text-caption text-mute-soft">
        <Link className="underline underline-offset-4" to={`/campagnes/${encodeURIComponent(campaignId)}`}>
          {t('gm.table.back')}
        </Link>
        <span className="type-title text-[15px] text-chalk">{state.preview.title}</span>
        <span className="type-label text-chalk">{t('gm.table.players')}</span>
        <span className="flex-1" />
        {characters.length > 0 && (
          <span>{t('gm.table.validatedCount', { validated, total: characters.length })}</span>
        )}
        <LiveIndicator status={live.status} />
      </header>

      <div className="grid flex-1 gap-4 p-4 lg:grid-cols-[300px_1fr]">
        <section className="surface-slab order-2 flex flex-col gap-2 p-3 lg:order-none">
          <h2 className="type-label text-chalk">{t('gm.table.tableTitle')}</h2>
          {seats.length === 0 ? (
            <p className="text-body text-mute">{t('gm.table.empty')}</p>
          ) : (
            <ul className="flex flex-col gap-1.5">
              {seats.map((seat) => (
                <SeatRow
                  key={seat.id}
                  seat={seat}
                  online={live.presence.players.includes(seat.id)}
                  current={seat.character !== null && reviewing === seat.character.id}
                  confirming={confirming === seat.id}
                  onOpen={() => seat.character && setView({ kind: 'review', characterId: seat.character.id })}
                  onRemove={() => (confirming === seat.id ? void remove(seat) : setConfirming(seat.id))}
                />
              ))}
            </ul>
          )}
          {actionError && <p role="alert">{t('gm.table.error')}</p>}
          <p className="mt-auto text-caption text-mute">{t('gm.table.footNote')}</p>
          <div className="flex flex-col gap-2">
            <CardButton
              variant="dark"
              size="small"
              icon="link"
              title={t('gm.table.inviteTitle')}
              aria-pressed={view.kind === 'invite'}
              onClick={() => setView({ kind: 'invite' })}
            />
            {validated > 0 && (
              <CardButton
                variant="dark"
                size="small"
                icon="shield"
                title={t('gm.sheets.title')}
                aria-pressed={view.kind === 'sheets'}
                onClick={() => setView({ kind: 'sheets' })}
              />
            )}
            <CardButton
              variant="dark"
              size="small"
              icon="eye"
              title={t('gm.hooks.title')}
              aria-pressed={view.kind === 'hooks'}
              onClick={() => setView({ kind: 'hooks' })}
            />
          </div>
        </section>

        <div className="flex min-w-0 flex-col">
          {view.kind === 'invite' && (
            <TableInvite
              campaignId={campaignId}
              preview={state.preview}
              initialInvite={state.invite}
              seats={seats}
            />
          )}
          {view.kind === 'review' && (
            <CharacterReview
              key={view.characterId}
              campaignId={campaignId}
              characterId={view.characterId}
              refreshKey={reviewVersion}
              onDecided={() => void loadSeats()}
            />
          )}
          {view.kind === 'hooks' && <SecretHooks campaignId={campaignId} seats={seats} />}
          {view.kind === 'sheets' && <PlaySheets campaignId={campaignId} refreshKey={tableVersion} />}
        </div>
      </div>
    </main>
  )
}

function seatTone(seat: Seat): SeatTone {
  switch (seat.character?.status) {
    case undefined:
      return 'off'
    case 'validated':
      return 'ok'
    case 'submitted':
      return 'todo'
    case 'draft':
    case 'returned':
    case 'fallen':
      return 'wait'
  }
}

function SeatRow({
  seat,
  online,
  current,
  confirming,
  onOpen,
  onRemove,
}: {
  seat: Seat
  online: boolean
  current: boolean
  confirming: boolean
  onOpen: () => void
  onRemove: () => void
}) {
  const { t } = useTranslation()
  const c = seat.character
  const tone = seatTone(seat)
  const statusKey = c ? (c.resubmitted ? 'resubmitted' : c.status) : 'spectator'
  const line = c
    ? c.name
      ? [c.name, c.className].filter(Boolean).join(' · ')
      : t('gm.table.creating')
    : t('gm.table.watching')
  const presence = online
    ? t('gm.table.online')
    : t('gm.table.seen', { date: dateFormat.format(new Date(seat.lastSeenAt)) })
  const body = (
    <>
      <span
        className={cn(
          'grid h-14 w-11 flex-none place-items-end justify-center overflow-hidden rounded-[9px] border-2',
          c?.look
            ? 'border-line-strong bg-linear-to-b from-surface-raised to-well shadow-[0_3px_0_var(--color-black)]'
            : 'border-dashed border-line-strong',
        )}
      >
        {c?.look && <Sprite look={c.look} scale={2} />}
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <b className="text-body">{seat.nickname}</b>
        <small className="truncate text-caption text-mute">
          {line} · {presence}
        </small>
      </span>
      <span
        className={cn(
          'flex-none rounded-md px-1.5 py-1 text-[10px] font-bold tracking-[0.08em] whitespace-nowrap uppercase',
          tone === 'ok' && 'bg-ivory text-ink',
          tone === 'todo' && 'bg-stat-init text-ink',
          tone === 'wait' && 'border-[1.5px] border-dashed border-line-dashed text-mute-soft',
          tone === 'off' && 'border border-line text-mute',
        )}
      >
        {t(`gm.table.status.${statusKey}`)}
      </span>
    </>
  )
  return (
    <li
      className={cn(
        'flex flex-col gap-1 rounded-button border border-transparent bg-well px-3 py-2.5',
        current && 'border-chalk bg-surface-raised',
      )}
    >
      {c ? (
        <button
          type="button"
          className="flex w-full items-center gap-3 text-left"
          aria-current={current || undefined}
          onClick={onOpen}
        >
          {body}
        </button>
      ) : (
        <div className="flex items-center gap-3">{body}</div>
      )}
      <Button variant="ghost" size="sm" className="self-end" onClick={onRemove}>
        {confirming ? t('gm.table.confirmRemove', { nickname: seat.nickname }) : t('gm.table.remove')}
      </Button>
    </li>
  )
}
