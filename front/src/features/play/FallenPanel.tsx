import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { ApiError } from '@/lib/api'
import { newCharacter, sayLastWords, type PlayerHome } from '@/lib/play'

const LAST_WORDS_MAX = 280

/**
 * After the GM confirmed my character's death (player/face-death,
 * planche « Mourir »): their last words, read at the table and kept in
 * the journal; then I stay — I watch the rest of the evening, or I
 * create another character, who enters at the group's level.
 */
export function FallenPanel({
  campaignId,
  fallen,
  onHome,
}: {
  campaignId: string
  fallen: NonNullable<PlayerHome['fallen']>
  onHome: (home: PlayerHome) => void
}) {
  const { t } = useTranslation()
  const [words, setWords] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function run(call: () => Promise<PlayerHome>) {
    setBusy(true)
    setError(null)
    try {
      onHome(await call())
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <section className="surface-slab flex flex-col gap-2 p-3.5">
        <span className="type-title text-[22px]">{t('play.fallen.title', { name: fallen.name })}</span>
        {fallen.lastWords ? (
          <>
            <span className="type-label">{t('play.fallen.said', { name: fallen.name })}</span>
            <blockquote className="type-narration text-[18px] leading-snug">« {fallen.lastWords} »</blockquote>
            <span className="text-caption text-mute">{t('play.fallen.kept')}</span>
          </>
        ) : (
          <form
            className="flex flex-col gap-2"
            onSubmit={(e) => {
              e.preventDefault()
              if (words.trim()) void run(() => sayLastWords(campaignId, words.trim()))
            }}
          >
            <label className="flex flex-col gap-1">
              <span className="type-label">{t('play.fallen.ask', { name: fallen.name })}</span>
              <textarea
                value={words}
                onChange={(e) => setWords(e.target.value)}
                maxLength={LAST_WORDS_MAX}
                rows={3}
                className="rounded-button border border-line bg-table p-2 text-body"
              />
            </label>
            <CardButton
              type="submit"
              disabled={busy || !words.trim()}
              title={t('play.fallen.say')}
              subtitle={t('play.fallen.sayHint')}
            />
          </form>
        )}
      </section>
      <section className="flex flex-col gap-2">
        <span className="type-title text-[20px]">{t('play.fallen.next')}</span>
        <div className="rounded-button border border-line p-3">
          <b className="text-body">{t('play.fallen.watch')}</b>
          <p className="text-caption text-chalk-soft">{t('play.fallen.watchHint')}</p>
        </div>
        <CardButton
          disabled={busy}
          title={t('play.fallen.create')}
          subtitle={t('play.fallen.createHint')}
          onClick={() => void run(() => newCharacter(campaignId))}
        />
      </section>
      {error && (
        <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
          {t(`play.fallen.errors.${error}`, { defaultValue: t('play.fallen.errors.UNEXPECTED') })}
        </p>
      )}
    </div>
  )
}
