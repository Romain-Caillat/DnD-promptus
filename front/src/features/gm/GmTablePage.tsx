import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useParams } from 'react-router'
import { Button } from '@/components/ui/button'
import { ApiError } from '@/lib/api'
import { cn } from '@/lib/utils'
import {
  closeInvite,
  fetchInvite,
  fetchPlayerPreview,
  forgetCode,
  joinLink,
  listSeats,
  mintInvite,
  rememberCode,
  rememberedCode,
  removeSeat,
  type InviteStatus,
  type PlayerPreview,
  type Seat,
} from '@/lib/table'

/** How often the table refreshes while the GM waits for players. */
const SEATS_POLL_MS = 5000
/** A player seen this recently is shown online. */
const ONLINE_MS = 2 * 60 * 1000

const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long', timeStyle: 'short' })

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; preview: PlayerPreview; invite: InviteStatus | null }

/**
 * `/campagnes/:campaignId` — the GM invites their table (board
 * « Inviter », moments 1 and 2): the campaign's link, a message ready to
 * paste on Discord, and the table filling up as players arrive.
 */
export function GmTablePage() {
  const { t } = useTranslation()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [code, setCode] = useState<string | null>(null)
  const [message, setMessage] = useState('')
  const [seats, setSeats] = useState<Seat[]>([])
  // When the seats were read: "online" is judged against it, not render time.
  const [polledAt, setPolledAt] = useState(0)
  const [copied, setCopied] = useState<'gm.table.copied' | 'gm.table.copyFailed' | null>(null)
  const [actionError, setActionError] = useState(false)
  const [confirming, setConfirming] = useState<string | null>(null)

  const draftMessage = useCallback(
    (preview: PlayerPreview, link: string) =>
      t('gm.table.message', { title: preview.title, hook: preview.playerHook, link }),
    [t],
  )

  useEffect(() => {
    let live = true
    Promise.all([fetchPlayerPreview(campaignId), fetchInvite(campaignId)]).then(
      ([preview, invite]) => {
        if (!live) return
        const known = rememberedCode(campaignId, invite)
        setState({ kind: 'ready', preview, invite })
        setCode(known)
        if (known) setMessage(draftMessage(preview, joinLink(known)))
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
  }, [campaignId, draftMessage])

  // The table fills up on its own: polled until realtime
  // (`session/stream-live-changes`) says when to refetch.
  useEffect(() => {
    if (state.kind !== 'ready') return
    let live = true
    const refresh = () =>
      listSeats(campaignId).then(
        (list) => {
          if (!live) return
          setSeats(list)
          setPolledAt(Date.now())
        },
        () => {
          // Kept as it was; the next poll tries again.
        },
      )
    void refresh()
    const timer = setInterval(() => void refresh(), SEATS_POLL_MS)
    return () => {
      live = false
      clearInterval(timer)
    }
  }, [campaignId, state.kind])

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />

  async function mint() {
    if (state.kind !== 'ready') return
    setActionError(false)
    setCopied(null)
    try {
      const minted = await mintInvite(campaignId)
      rememberCode(campaignId, minted.code, minted.createdAt)
      setCode(minted.code)
      setMessage(draftMessage(state.preview, joinLink(minted.code)))
      setState({ ...state, invite: { createdAt: minted.createdAt, expiresAt: minted.expiresAt } })
    } catch {
      setActionError(true)
    }
  }

  async function close() {
    if (state.kind !== 'ready') return
    setActionError(false)
    try {
      await closeInvite(campaignId)
      forgetCode(campaignId)
      setCode(null)
      setState({ ...state, invite: null })
    } catch {
      setActionError(true)
    }
  }

  async function copy() {
    try {
      await navigator.clipboard.writeText(message)
      setCopied('gm.table.copied')
    } catch {
      setCopied('gm.table.copyFailed')
    }
  }

  async function remove(seat: Seat) {
    setActionError(false)
    try {
      await removeSeat(campaignId, seat.id)
      setSeats((list) => list.filter((s) => s.id !== seat.id))
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

  const { preview, invite } = state
  const link = code ? joinLink(code) : null

  return (
    <main className="surface-table flex min-h-dvh flex-col text-chalk">
      <header className="flex flex-wrap items-center gap-4 border-b border-line px-5 py-3 text-caption text-mute-soft">
        <Link className="underline underline-offset-4" to="/">
          {t('gm.table.back')}
        </Link>
        <span className="type-title text-[15px] text-chalk">{preview.title}</span>
        <span className="type-label text-chalk">{t('gm.table.players')}</span>
      </header>

      <div className="grid flex-1 gap-4 p-4 lg:grid-cols-[300px_1fr]">
        <section className="surface-slab flex flex-col gap-2 p-3 lg:order-none order-2">
          <h2 className="type-label text-chalk">{t('gm.table.tableTitle')}</h2>
          {seats.length === 0 ? (
            <p className="text-body text-mute">{t('gm.table.empty')}</p>
          ) : (
            <ul className="flex flex-col gap-1.5">
              {seats.map((seat) => (
                <SeatRow
                  key={seat.id}
                  seat={seat}
                  now={polledAt}
                  confirming={confirming === seat.id}
                  onRemove={() => (confirming === seat.id ? void remove(seat) : setConfirming(seat.id))}
                />
              ))}
            </ul>
          )}
          <p className="mt-auto text-caption text-mute">{t('gm.table.footNote')}</p>
        </section>

        <section className="flex flex-col gap-4">
          <h1 className="type-title text-heading">{t('gm.table.inviteTitle')}</h1>
          <div className="flex flex-col gap-2">
            <span className="type-label">{t('gm.table.linkLabel')}</span>
            <div className="flex flex-wrap items-center gap-3 rounded-button border border-line-strong bg-well p-3">
              {link ? (
                <input
                  className="min-w-0 flex-1 bg-transparent text-body text-chalk"
                  value={link}
                  readOnly
                  aria-label={t('gm.table.linkField')}
                  onFocus={(e) => e.currentTarget.select()}
                />
              ) : (
                <p className="min-w-0 flex-1 text-body text-chalk-soft">
                  {invite
                    ? t('gm.table.hiddenLink', { date: dateFormat.format(new Date(invite.expiresAt)) })
                    : t('gm.table.noLink')}
                </p>
              )}
              <Button variant="outline" onClick={() => void mint()}>
                {invite ? t('gm.table.regenerate') : t('gm.table.create')}
              </Button>
              {invite && (
                <Button variant="ghost" onClick={() => void close()}>
                  {t('gm.table.close')}
                </Button>
              )}
            </div>
            {invite && link && (
              <p className="text-caption text-mute">
                {t('gm.table.validUntil', { date: dateFormat.format(new Date(invite.expiresAt)) })}
              </p>
            )}
          </div>

          {link && (
            <label className="flex flex-col gap-2">
              <span className="type-label">{t('gm.table.messageLabel')}</span>
              <textarea
                className="min-h-36 rounded-button border border-line-strong bg-well p-3.5 text-body leading-relaxed text-chalk-soft"
                value={message}
                onChange={(e) => setMessage(e.target.value)}
              />
            </label>
          )}
          {actionError && <p role="alert">{t('gm.table.error')}</p>}
        </section>
      </div>

      <footer className="flex flex-wrap items-center justify-end gap-4 px-5 pb-5">
        {copied && (
          <p role="status" className="text-body text-chalk-soft">
            {t(copied)}
          </p>
        )}
        <button
          type="button"
          className="button-card w-full max-w-[460px] disabled:opacity-50"
          disabled={!link}
          onClick={() => void copy()}
        >
          <span className="card-frame flex min-h-10 flex-col justify-center px-3.5 py-1.5">
            <span className="type-title text-card-title">{t('gm.table.copy')}</span>
            <span className="mt-0.5 text-[11px] font-semibold text-(color:--sub-color)">{t('gm.table.copySub')}</span>
          </span>
        </button>
      </footer>
    </main>
  )
}

function SeatRow({
  seat,
  now,
  confirming,
  onRemove,
}: {
  seat: Seat
  now: number
  confirming: boolean
  onRemove: () => void
}) {
  const { t } = useTranslation()
  const online = now - new Date(seat.lastSeenAt).getTime() < ONLINE_MS
  const statusKey = seat.character ? seat.character.status : 'spectator'
  const line = seat.character
    ? seat.character.name || t('gm.table.creating')
    : t('gm.table.watching')
  return (
    <li className="flex items-center gap-3 rounded-button bg-well px-3 py-2.5">
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <b className="text-body">{seat.nickname}</b>
        <small className="truncate text-caption text-mute">
          {line} · {online ? t('gm.table.online') : t('gm.table.seen', { date: dateFormat.format(new Date(seat.lastSeenAt)) })}
        </small>
      </div>
      <span
        className={cn(
          'rounded-md px-1.5 py-1 text-[10px] font-bold tracking-[0.08em] uppercase',
          statusKey === 'validated' && 'bg-ivory text-ink',
          statusKey === 'submitted' && 'bg-stat-init text-ink',
          statusKey === 'returned' && 'border-[1.5px] border-stat-atk text-stat-atk',
          (statusKey === 'draft' || statusKey === 'spectator') && 'border-[1.5px] border-dashed border-line-dashed text-mute-soft',
        )}
      >
        {t(`gm.table.status.${statusKey}`)}
      </span>
      <Button
        variant="ghost"
        size="sm"
        onClick={onRemove}
      >
        {confirming ? t('gm.table.confirmRemove', { nickname: seat.nickname }) : t('gm.table.remove')}
      </Button>
    </li>
  )
}
