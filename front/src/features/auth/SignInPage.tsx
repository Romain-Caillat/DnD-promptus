import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { Button } from '@/components/ui/button'
import { ApiError } from '@/lib/api'
import { authErrorKey, requestCode, verifyCode, type AuthErrorKey } from '@/lib/auth'

/** The address typed last on this device, to spare typing it again. */
const LAST_EMAIL = 'promptus.gm.email'

function lastEmail(): string {
  try {
    return localStorage.getItem(LAST_EMAIL) ?? ''
  } catch {
    return ''
  }
}

function rememberEmail(email: string) {
  try {
    localStorage.setItem(LAST_EMAIL, email)
  } catch {
    // Private browsing: typing it again is fine.
  }
}

type Step = 'email' | 'code' | 'name'

const FIELD = 'h-10 w-full pixel-field px-3'

/**
 * `/connexion` — a GM signs in with their email: a code is sent there,
 * typed back here. The first time, they also give the name players will
 * see, and the account is created.
 */
export function SignInPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [step, setStep] = useState<Step>('email')
  const [email, setEmail] = useState(lastEmail)
  const [code, setCode] = useState('')
  const [name, setName] = useState('')
  const [pending, setPending] = useState(false)
  const [error, setError] = useState<AuthErrorKey | null>(null)
  const [resent, setResent] = useState(false)

  async function run(action: () => Promise<void>) {
    setPending(true)
    setError(null)
    try {
      await action()
    } catch (err) {
      setError(authErrorKey(err))
    } finally {
      setPending(false)
    }
  }

  function sendCode(e: FormEvent) {
    e.preventDefault()
    void run(async () => {
      await requestCode(email)
      rememberEmail(email.trim())
      setCode('')
      setResent(false)
      setStep('code')
    })
  }

  function resend() {
    void run(async () => {
      await requestCode(email)
      setResent(true)
    })
  }

  function signIn(e: FormEvent) {
    e.preventDefault()
    void run(async () => {
      try {
        await verifyCode(email, code, step === 'name' ? name : undefined)
      } catch (err) {
        if (err instanceof ApiError && err.code === 'DISPLAY_NAME_REQUIRED') {
          setStep('name')
          return
        }
        throw err
      }
      navigate('/', { replace: true })
    })
  }

  function changeEmail() {
    setStep('email')
    setError(null)
  }

  return (
    <main className="mx-auto flex min-h-dvh max-w-md flex-col justify-center gap-6 p-6">
      <header className="flex flex-col gap-2">
        <h1 className="type-title text-heading-lg">{t('auth.signIn.title')}</h1>
        <p className="text-muted-foreground">
          {step === 'email'
            ? t('auth.signIn.intro')
            : step === 'code'
              ? t('auth.signIn.codeSent', { email: email.trim() })
              : t('auth.signIn.newAccount')}
        </p>
      </header>

      {step === 'email' ? (
        <form className="flex flex-col items-start gap-3" onSubmit={sendCode}>
          <label className="flex w-full flex-col gap-1">
            <span className="text-sm">{t('auth.signIn.email')}</span>
            <input
              className={FIELD}
              type="email"
              inputMode="email"
              autoComplete="email"
              required
              value={email}
              onChange={(e) => setEmail(e.currentTarget.value)}
            />
          </label>
          <Button type="submit" disabled={pending}>
            {pending ? t('auth.signIn.pending') : t('auth.signIn.sendCode')}
          </Button>
        </form>
      ) : (
        <form className="flex flex-col items-start gap-3" onSubmit={signIn}>
          <label className="flex w-full flex-col gap-1">
            <span className="text-sm">{t('auth.signIn.code')}</span>
            <input
              className={`${FIELD} font-mono tracking-[0.4em]`}
              inputMode="numeric"
              autoComplete="one-time-code"
              pattern="[0-9]{6}"
              maxLength={6}
              required
              readOnly={step === 'name'}
              value={code}
              onChange={(e) => setCode(e.currentTarget.value.replace(/\D/g, ''))}
            />
          </label>
          {step === 'name' && (
            <label className="flex w-full flex-col gap-1">
              <span className="text-sm">{t('auth.signIn.displayName')}</span>
              <input
                className={FIELD}
                autoComplete="nickname"
                maxLength={60}
                required
                value={name}
                onChange={(e) => setName(e.currentTarget.value)}
              />
            </label>
          )}
          <Button type="submit" disabled={pending}>
            {pending
              ? t('auth.signIn.pending')
              : step === 'name'
                ? t('auth.signIn.createAccount')
                : t('auth.signIn.submit')}
          </Button>
          <div className="flex flex-wrap gap-x-4 gap-y-1 text-sm">
            <button
              type="button"
              className="underline underline-offset-4"
              onClick={resend}
              disabled={pending}
            >
              {t('auth.signIn.resend')}
            </button>
            <button type="button" className="underline underline-offset-4" onClick={changeEmail}>
              {t('auth.signIn.changeEmail')}
            </button>
          </div>
          {resent && !error && <p role="status">{t('auth.signIn.resent')}</p>}
        </form>
      )}
      {error && <p role="alert">{t(error)}</p>}
    </main>
  )
}
