import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate, useParams } from 'react-router'
import { ApiError } from '@/lib/api'
import { creatorPath } from '@/lib/creator'
import { cn } from '@/lib/utils'
import {
  fetchInvitation,
  fetchPlayerHome,
  joinCampaign,
  playPath,
  type Invitation,
  type PlayerHome,
  type Role,
} from '@/lib/play'

type LoadState =
  | { kind: 'loading' }
  | { kind: 'dead' }
  | { kind: 'error' }
  | { kind: 'ready'; invitation: Invitation; seat: PlayerHome | null }

type JoinError = 'join.errors.nickname' | 'join.errors.taken' | 'join.dead' | 'join.errors.generic'

function joinErrorKey(err: unknown): JoinError {
  if (err instanceof ApiError) {
    if (err.code === 'INVALID_NICKNAME') return 'join.errors.nickname'
    if (err.code === 'NICKNAME_TAKEN') return 'join.errors.taken'
    if (err.code === 'INVITE_NOT_FOUND') return 'join.dead'
  }
  return 'join.errors.generic'
}

/**
 * `/rejoindre/:code` — a player opens the GM's link on their phone, picks
 * a nickname, then creates a character or watches. No account: the
 * server leaves a token in a cookie only this campaign's routes receive.
 * A browser that already has its seat is offered to resume it.
 */
export function JoinPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { code = '' } = useParams()
  const [state, setState] = useState<LoadState>({ kind: 'loading' })
  const [nickname, setNickname] = useState('')
  const [pending, setPending] = useState(false)
  const [error, setError] = useState<JoinError | null>(null)

  useEffect(() => {
    let live = true
    async function load() {
      const invitation = await fetchInvitation(code)
      if (!invitation) return { kind: 'dead' } as const
      const seat = await fetchPlayerHome(invitation.campaignId)
      return { kind: 'ready', invitation, seat } as const
    }
    load().then(
      (next) => {
        if (live) setState(next)
      },
      () => {
        if (live) setState({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [code])

  async function join(role: Role) {
    if (state.kind !== 'ready') return
    if (nickname.trim() === '') {
      setError('join.errors.nickname')
      return
    }
    setPending(true)
    setError(null)
    try {
      await joinCampaign(code, nickname, role)
      // « Create my character » opens the creator straight away.
      const id = state.invitation.campaignId
      navigate(role === 'player' ? creatorPath(id) : playPath(id), { replace: true })
    } catch (err) {
      setError(joinErrorKey(err))
      setPending(false)
    }
  }

  return (
    <main className="surface-table mx-auto flex min-h-dvh max-w-md flex-col text-chalk">
      {state.kind === 'loading' && (
        <p role="status" className="p-4">
          {t('join.loading')}
        </p>
      )}
      {state.kind === 'dead' && (
        <p role="alert" className="p-4">
          {t('join.dead')}
        </p>
      )}
      {state.kind === 'error' && (
        <p role="alert" className="p-4">
          {t('join.errors.generic')}
        </p>
      )}
      {state.kind === 'ready' && (
        <>
          <p className="mx-3 mt-4 rounded-button bg-ivory px-3.5 py-2.5 text-body font-bold text-ink shadow-ivory-flat">
            {t('join.banner')}
          </p>
          <section className="flex flex-1 flex-col gap-3 p-4">
            <span className="type-label">{t('join.invites', { gm: state.invitation.gmName })}</span>
            <h1 className="type-title text-heading">{state.invitation.title}</h1>
            {state.invitation.playerHook && (
              <p className="type-narration text-[18px] leading-snug text-chalk-soft">
                {state.invitation.playerHook}
              </p>
            )}
            {state.seat ? (
              <p className="text-body text-chalk-soft">
                {t('join.already', { nickname: state.seat.me.nickname })}
              </p>
            ) : (
              <>
                <label className="flex flex-col gap-1.5">
                  <span className="type-label">{t('join.nickname')}</span>
                  <input
                    className="pixel-field px-3.5 py-3 text-base"
                    value={nickname}
                    maxLength={40}
                    autoComplete="nickname"
                    onChange={(e) => setNickname(e.target.value)}
                  />
                </label>
                <p className="text-caption text-mute">{t('join.noAccount')}</p>
              </>
            )}
            {error && (
              <p role="alert" className="text-body text-chalk">
                {t(error)}
              </p>
            )}
          </section>
          <footer className="flex flex-col gap-2.5 px-3.5 pt-2.5 pb-8">
            {state.seat ? (
              <ChoiceButton
                title={t('join.resume')}
                sub={t('join.resumeSub')}
                onClick={() => navigate(playPath(state.invitation.campaignId))}
              />
            ) : (
              <>
                <ChoiceButton
                  title={pending ? t('join.pending') : t('join.create')}
                  sub={t('join.createSub')}
                  disabled={pending}
                  onClick={() => void join('player')}
                />
                <ChoiceButton
                  dark
                  title={t('join.watch')}
                  sub={t('join.watchSub')}
                  disabled={pending}
                  onClick={() => void join('spectator')}
                />
              </>
            )}
          </footer>
        </>
      )}
    </main>
  )
}

/** A pixel key (`button-card`): ivory for the main choice. */
function ChoiceButton(props: {
  title: string
  sub: string
  dark?: boolean
  disabled?: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      className={cn('button-card w-full disabled:opacity-60', props.dark && 'button-card-dark')}
      disabled={props.disabled}
      onClick={props.onClick}
    >
      <span className="card-frame flex min-h-10 flex-col justify-center px-3 py-2">
        <span className="type-key text-card-title">{props.title}</span>
        <span className="mt-0.5 text-[11px] font-semibold text-(color:--sub-color)">{props.sub}</span>
      </span>
    </button>
  )
}
