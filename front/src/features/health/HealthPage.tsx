import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { fetchHealth, type HealthStatus } from '@/lib/api'

const STATUS_KEY = {
  ok: 'health.ok',
  'database-down': 'health.databaseDown',
  unreachable: 'health.unreachable',
} as const satisfies Record<HealthStatus, string>

/**
 * Placeholder home page until the first real screen lands: proves the
 * whole chain (UI → Vite proxy → Axum → Postgres) answers.
 */
export function HealthPage() {
  const { t } = useTranslation()
  const [status, setStatus] = useState<HealthStatus | null>(null)
  // Bumped by "Réessayer" to re-run the check.
  const [attempt, setAttempt] = useState(0)

  useEffect(() => {
    const controller = new AbortController()
    fetchHealth(controller.signal).then(setStatus, () => {
      // Aborted on unmount or by a newer attempt — nothing to show.
    })
    return () => controller.abort()
  }, [attempt])

  return (
    <main className="mx-auto flex min-h-dvh max-w-md flex-col justify-center gap-6 p-6">
      <header className="flex flex-col gap-2">
        <h1 className="type-title text-heading-lg">{t('app.name')}</h1>
        <p className="text-muted-foreground">{t('app.tagline')}</p>
      </header>
      <section className="flex flex-col items-start gap-3" aria-live="polite">
        <p role="status">{status === null ? t('health.checking') : t(STATUS_KEY[status])}</p>
        {status !== null && status !== 'ok' && (
          <Button
            variant="outline"
            onClick={() => {
              setStatus(null)
              setAttempt((n) => n + 1)
            }}
          >
            {t('health.retry')}
          </Button>
        )}
      </section>
    </main>
  )
}
