import { useEffect, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { Button } from '@/components/ui/button'
import { authErrorKey, fetchNeedsSetup, registerWithPasskey, type AuthErrorKey } from '@/lib/auth'
import { passkeysSupported } from '@/lib/webauthn'

const inputClass = 'h-9 w-full pixel-field px-3 text-base'

/**
 * `/inscription` — create a GM account with a passkey. The code is the
 * setup code of a fresh server (first account) or comes from another
 * GM's invitation link (`?code=`).
 */
export function RegisterPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const [code, setCode] = useState(() => params.get('code') ?? '')
  const [displayName, setDisplayName] = useState('')
  const [pending, setPending] = useState(false)
  const [error, setError] = useState<AuthErrorKey | null>(null)
  const [needsSetup, setNeedsSetup] = useState(false)
  const supported = passkeysSupported()

  useEffect(() => {
    let live = true
    fetchNeedsSetup().then(
      (value) => {
        if (live) setNeedsSetup(value)
      },
      () => {
        // Submitting reports an unreachable server itself.
      },
    )
    return () => {
      live = false
    }
  }, [])

  async function submit(event: FormEvent) {
    event.preventDefault()
    setPending(true)
    setError(null)
    try {
      await registerWithPasskey(code.trim(), displayName.trim())
      navigate('/', { replace: true })
    } catch (err) {
      setError(authErrorKey(err))
      setPending(false)
    }
  }

  return (
    <main className="mx-auto flex min-h-dvh max-w-md flex-col justify-center gap-6 p-6">
      <header className="flex flex-col gap-2">
        <h1 className="type-title text-heading-lg">{t('auth.register.title')}</h1>
        <p className="text-muted-foreground">{t('auth.register.intro')}</p>
        <p className="text-sm text-muted-foreground">
          {needsSetup ? t('auth.register.setupHint') : t('auth.register.inviteHint')}
        </p>
      </header>
      {supported ? (
        <form className="flex flex-col gap-4" onSubmit={(e) => void submit(e)}>
          <label className="flex flex-col gap-1.5">
            <span className="type-label text-chalk-soft">{t('auth.register.code')}</span>
            <input
              className={inputClass}
              value={code}
              onChange={(e) => setCode(e.target.value)}
              autoComplete="off"
              required
            />
          </label>
          <label className="flex flex-col gap-1.5">
            <span className="type-label text-chalk-soft">{t('auth.register.displayName')}</span>
            <input
              className={inputClass}
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              maxLength={60}
              autoComplete="nickname"
              required
            />
          </label>
          <Button type="submit" disabled={pending}>
            {pending ? t('auth.register.pending') : t('auth.register.submit')}
          </Button>
          {error && <p role="alert">{t(error)}</p>}
        </form>
      ) : (
        <p role="alert">{t('auth.errors.unsupported')}</p>
      )}
      <Link className="text-sm underline underline-offset-4" to="/connexion">
        {t('auth.register.haveAccount')}
      </Link>
    </main>
  )
}
