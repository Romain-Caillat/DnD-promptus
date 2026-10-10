import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { createInvite, inviteLink, listInvites, revokeInvite, type GmInvite } from '@/lib/auth'

const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long', timeStyle: 'short' })

/** Invite another GM: mint a single-use link, list and revoke pending ones. */
export function InvitesPanel() {
  const { t } = useTranslation()
  const [invites, setInvites] = useState<GmInvite[]>([])
  const [link, setLink] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  const [error, setError] = useState(false)

  const refresh = useCallback(async () => {
    try {
      setInvites(await listInvites())
    } catch {
      setError(true)
    }
  }, [])

  useEffect(() => {
    let live = true
    listInvites().then(
      (list) => {
        if (live) setInvites(list)
      },
      () => {
        if (live) setError(true)
      },
    )
    return () => {
      live = false
    }
  }, [])

  async function create() {
    setError(false)
    setCopied(false)
    try {
      const minted = await createInvite()
      setLink(inviteLink(minted.code))
      await refresh()
    } catch {
      setError(true)
    }
  }

  async function revoke(id: string) {
    setError(false)
    try {
      await revokeInvite(id)
      await refresh()
    } catch {
      setError(true)
    }
  }

  async function copy(value: string) {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
    } catch {
      // The link stays selectable in the field.
    }
  }

  return (
    <section className="flex flex-col gap-4">
      <div className="flex flex-col gap-1">
        <h2 className="text-lg font-semibold">{t('gm.invites.title')}</h2>
        <p className="text-sm text-muted-foreground">{t('gm.invites.intro')}</p>
      </div>
      <div>
        <Button onClick={() => void create()}>{t('gm.invites.create')}</Button>
      </div>
      {link && (
        <div className="flex flex-col gap-2">
          <p className="text-sm">{t('gm.invites.created')}</p>
          <div className="flex flex-wrap gap-2">
            <input
              className="h-9 min-w-0 flex-1 pixel-field px-3 text-sm"
              value={link}
              readOnly
              aria-label={t('gm.invites.linkLabel')}
              onFocus={(e) => e.currentTarget.select()}
            />
            <Button variant="outline" onClick={() => void copy(link)}>
              {copied ? t('gm.invites.copied') : t('gm.invites.copy')}
            </Button>
          </div>
        </div>
      )}
      {error && <p role="alert">{t('gm.invites.error')}</p>}
      <div className="flex flex-col gap-2">
        <h3 className="font-medium">{t('gm.invites.pending')}</h3>
        {invites.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('gm.invites.none')}</p>
        ) : (
          <ul className="flex flex-col gap-2">
            {invites.map((invite) => (
              <li key={invite.id} className="flex items-center justify-between gap-3 text-sm">
                <span>
                  {t('gm.invites.expires', { date: dateFormat.format(new Date(invite.expiresAt)) })}
                </span>
                <Button variant="ghost" size="sm" onClick={() => void revoke(invite.id)}>
                  {t('gm.invites.revoke')}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  )
}
