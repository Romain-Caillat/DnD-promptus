import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { Button } from '@/components/ui/button'
import { authErrorKey, fetchNeedsSetup, signInWithPasskey, type AuthErrorKey } from '@/lib/auth'
import { passkeysSupported } from '@/lib/webauthn'

/** `/connexion` — a GM signs in with a passkey. */
export function SignInPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
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
        // The sign-in button reports an unreachable server itself.
      },
    )
    return () => {
      live = false
    }
  }, [])

  async function submit() {
    setPending(true)
    setError(null)
    try {
      await signInWithPasskey()
      navigate('/', { replace: true })
    } catch (err) {
      setError(authErrorKey(err))
      setPending(false)
    }
  }

  return (
    <main className="mx-auto flex min-h-dvh max-w-md flex-col justify-center gap-6 p-6">
      <header className="flex flex-col gap-2">
        <h1 className="text-3xl font-semibold tracking-tight">{t('auth.signIn.title')}</h1>
        <p className="text-muted-foreground">{t('auth.signIn.intro')}</p>
      </header>
      {needsSetup ? (
        <section className="flex flex-col items-start gap-3">
          <p>{t('auth.signIn.setupNeeded')}</p>
          <Link className="underline underline-offset-4" to="/inscription">
            {t('auth.signIn.setupLink')}
          </Link>
        </section>
      ) : (
        <section className="flex flex-col items-start gap-3">
          {supported ? (
            <Button onClick={() => void submit()} disabled={pending}>
              {pending ? t('auth.signIn.pending') : t('auth.signIn.submit')}
            </Button>
          ) : (
            <p role="alert">{t('auth.errors.unsupported')}</p>
          )}
          {error && <p role="alert">{t(error)}</p>}
          <Link className="text-sm underline underline-offset-4" to="/inscription">
            {t('auth.signIn.noAccount')}
          </Link>
        </section>
      )}
    </main>
  )
}
