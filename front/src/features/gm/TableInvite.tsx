import { useCallback, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { StatusBanner } from '@/components/game/StatusBanner'
import {
  closeInvite,
  forgetCode,
  joinLink,
  mintInvite,
  rememberCode,
  rememberedCode,
  type InviteStatus,
  type PlayerPreview,
  type Seat,
} from '@/lib/table'

const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long', timeStyle: 'short' })

/**
 * The campaign's invitation (board « Inviter », moments 1 and 2): the
 * link, a message ready to paste on Discord, and — once someone sent a
 * sheet — a banner pointing at it.
 */
export function TableInvite({
  campaignId,
  preview,
  initialInvite,
  seats,
}: {
  campaignId: string
  preview: PlayerPreview
  initialInvite: InviteStatus | null
  seats: Seat[]
}) {
  const { t } = useTranslation()
  const draftMessage = useCallback(
    (link: string) => t('gm.table.message', { title: preview.title, hook: preview.playerHook, link }),
    [t, preview],
  )
  const [invite, setInvite] = useState(initialInvite)
  const [code, setCode] = useState<string | null>(() => rememberedCode(campaignId, initialInvite))
  const [message, setMessage] = useState(() => (code ? draftMessage(joinLink(code)) : ''))
  const [copied, setCopied] = useState<'gm.table.copied' | 'gm.table.copyFailed' | null>(null)
  const [actionError, setActionError] = useState(false)

  async function mint() {
    setActionError(false)
    setCopied(null)
    try {
      const minted = await mintInvite(campaignId)
      rememberCode(campaignId, minted.code, minted.createdAt)
      setCode(minted.code)
      setMessage(draftMessage(joinLink(minted.code)))
      setInvite({ createdAt: minted.createdAt, expiresAt: minted.expiresAt })
    } catch {
      setActionError(true)
    }
  }

  async function close() {
    setActionError(false)
    try {
      await closeInvite(campaignId)
      forgetCode(campaignId)
      setCode(null)
      setInvite(null)
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

  const link = code ? joinLink(code) : null
  const waiting = seats.find((s) => s.character?.status === 'submitted')

  return (
    <div className="flex flex-1 flex-col gap-4">
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

      <footer className="mt-auto flex flex-wrap items-center justify-end gap-4">
        {waiting?.character && (
          <StatusBanner tone="you" className="min-w-[240px] flex-1">
            {t('gm.table.waiting', {
              nickname: waiting.nickname,
              name: waiting.character.name || t('gm.review.unnamed'),
            })}
          </StatusBanner>
        )}
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
    </div>
  )
}
