import { useCallback, useEffect, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  claudeCodeCommand,
  createToken,
  desktopConfig,
  listTokens,
  revokeToken,
  TOKEN_PLACEHOLDER,
  type ApiToken,
} from '@/lib/claudeAccess'

const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long', timeStyle: 'short' })

type Copied = 'token' | 'desktop' | 'code' | null

/**
 * « Accès pour Claude » (platform/connect-claude-mcp): the GM mints a
 * named token for the Claude on their own computer — shown once —, sees
 * when each was last used and revokes it; the help says how to declare
 * the local MCP server in Claude Desktop or Claude Code.
 */
export function ClaudeAccessPanel() {
  const { t } = useTranslation()
  const [tokens, setTokens] = useState<ApiToken[]>([])
  const [name, setName] = useState('')
  const [secret, setSecret] = useState<string | null>(null)
  const [copied, setCopied] = useState<Copied>(null)
  const [error, setError] = useState<'name' | 'failed' | null>(null)

  const refresh = useCallback(async () => {
    try {
      setTokens(await listTokens())
    } catch {
      setError('failed')
    }
  }, [])

  useEffect(() => {
    let live = true
    listTokens().then(
      (list) => {
        if (live) setTokens(list)
      },
      () => {
        if (live) setError('failed')
      },
    )
    return () => {
      live = false
    }
  }, [])

  async function create(event: FormEvent) {
    event.preventDefault()
    setError(null)
    setCopied(null)
    if (name.trim() === '') {
      setError('name')
      return
    }
    try {
      const minted = await createToken(name.trim())
      setSecret(minted.secret)
      setName('')
      await refresh()
    } catch {
      setError('failed')
    }
  }

  async function revoke(id: string) {
    setError(null)
    try {
      await revokeToken(id)
      await refresh()
    } catch {
      setError('failed')
    }
  }

  async function copy(value: string, what: Copied) {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(what)
    } catch {
      // The text stays selectable.
    }
  }

  const origin = window.location.origin
  const token = secret ?? TOKEN_PLACEHOLDER
  const desktop = desktopConfig(origin, token)
  const code = claudeCodeCommand(origin, token)

  return (
    <section className="flex flex-col gap-4" aria-labelledby="claude-access-title">
      <div className="flex flex-col gap-1">
        <h2 id="claude-access-title" className="text-lg font-semibold">
          {t('gm.claude.title')}
        </h2>
        <p className="text-sm text-muted-foreground">{t('gm.claude.intro')}</p>
      </div>
      <form className="flex flex-wrap gap-2" onSubmit={(e) => void create(e)}>
        <input
          className="h-9 min-w-0 flex-1 rounded-md border border-input bg-background px-3 text-sm"
          value={name}
          maxLength={60}
          onChange={(e) => setName(e.currentTarget.value)}
          aria-label={t('gm.claude.nameLabel')}
          placeholder={t('gm.claude.namePlaceholder')}
        />
        <Button type="submit">{t('gm.claude.create')}</Button>
      </form>
      {secret && (
        <div className="flex flex-col gap-2">
          <p className="text-sm">{t('gm.claude.created')}</p>
          <div className="flex flex-wrap gap-2">
            <input
              className="h-9 min-w-0 flex-1 rounded-md border border-input bg-background px-3 font-mono text-sm"
              value={secret}
              readOnly
              aria-label={t('gm.claude.tokenLabel')}
              onFocus={(e) => e.currentTarget.select()}
            />
            <Button variant="outline" onClick={() => void copy(secret, 'token')}>
              {copied === 'token' ? t('gm.claude.copied') : t('gm.claude.copy')}
            </Button>
          </div>
        </div>
      )}
      {error && <p role="alert">{error === 'name' ? t('gm.claude.nameRequired') : t('gm.claude.error')}</p>}
      <div className="flex flex-col gap-2">
        <h3 className="font-medium">{t('gm.claude.list')}</h3>
        {tokens.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('gm.claude.none')}</p>
        ) : (
          <ul className="flex flex-col gap-2">
            {tokens.map((item) => (
              <li key={item.id} className="flex items-center justify-between gap-3 text-sm">
                <span className="flex flex-col">
                  <span className="font-medium">{item.name}</span>
                  <span className="text-muted-foreground">
                    {t('gm.claude.meta', {
                      created: dateFormat.format(new Date(item.createdAt)),
                      used: item.lastUsedAt
                        ? t('gm.claude.lastUsed', { date: dateFormat.format(new Date(item.lastUsedAt)) })
                        : t('gm.claude.neverUsed'),
                    })}
                  </span>
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label={t('gm.claude.revokeLabel', { name: item.name })}
                  onClick={() => void revoke(item.id)}
                >
                  {t('gm.claude.revoke')}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </div>
      <details className="flex flex-col gap-3 text-sm">
        <summary className="cursor-pointer font-medium">{t('gm.claude.help.title')}</summary>
        <ol className="mt-2 flex list-decimal flex-col gap-3 pl-5">
          <li>{t('gm.claude.help.install')}</li>
          <li className="flex flex-col gap-2">
            <span>{t('gm.claude.help.desktop')}</span>
            <pre
              className="overflow-auto rounded-button border border-line bg-table p-2 text-[12px]"
              aria-label={t('gm.claude.help.desktopLabel')}
            >
              {desktop}
            </pre>
            <Button variant="outline" size="sm" className="self-start" onClick={() => void copy(desktop, 'desktop')}>
              {copied === 'desktop' ? t('gm.claude.copied') : t('gm.claude.copy')}
            </Button>
          </li>
          <li className="flex flex-col gap-2">
            <span>{t('gm.claude.help.code')}</span>
            <pre
              className="overflow-auto rounded-button border border-line bg-table p-2 text-[12px]"
              aria-label={t('gm.claude.help.codeLabel')}
            >
              {code}
            </pre>
            <Button variant="outline" size="sm" className="self-start" onClick={() => void copy(code, 'code')}>
              {copied === 'code' ? t('gm.claude.copied') : t('gm.claude.copy')}
            </Button>
          </li>
          <li>{t('gm.claude.help.paths')}</li>
          <li>{t('gm.claude.help.scope')}</li>
        </ol>
      </details>
    </section>
  )
}
