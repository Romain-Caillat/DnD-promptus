import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { Sprite } from '@/features/sprites/Sprite'
import { chooseNext, sayLastWords, type FallenView, type FateNext, type PlayerHome } from '@/lib/play'
import { cn } from '@/lib/utils'

/** As the server caps them (`players::fate::LAST_WORDS_MAX`). */
const WORDS_MAX = 280

const CHOICES: FateNext[] = ['watch', 'new', 'hook']

/**
 * player/face-death (planche « Mourir », moments 6 and 7): the dead
 * character, grey; their last words, written once and read by the whole
 * table; then what the player does now — watch the evening, make a new
 * character at the party's level (final), or wait for the GM's hook.
 * The player stays at the table whatever they choose.
 */
export function FallenCard({
  campaignId,
  fallen,
  onChanged,
}: {
  campaignId: string
  fallen: FallenView
  onChanged: (home: PlayerHome) => void
}) {
  const { t } = useTranslation()
  const [words, setWords] = useState('')
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)
  const name = fallen.name || t('play.unnamed')

  async function send(call: () => Promise<PlayerHome>) {
    setBusy(true)
    setFailed(false)
    try {
      onChanged(await call())
    } catch {
      setFailed(true)
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="flex flex-col gap-4" aria-label={name}>
      <div className="flex flex-col items-center gap-2 rounded-2xl border border-line bg-[radial-gradient(60%_40%_at_50%_30%,rgba(255,77,94,.12),transparent)] p-5 text-center">
        {fallen.look && (
          <span className="brightness-75 grayscale">
            <Sprite look={fallen.look} scale={4} label={name} />
          </span>
        )}
        <span className="type-title text-[22px]">{name}</span>
        <span className="type-label">
          {[fallen.className, t('play.fallen.kicker', { level: fallen.level })].filter(Boolean).join(' · ')}
        </span>
      </div>

      <div className="flex flex-col gap-2">
        <span className="type-label">{t('play.fallen.words')}</span>
        {fallen.lastWords ? (
          <>
            <blockquote className="rounded-xl border-[1.5px] border-chalk px-4 py-3 font-serif text-[19px] leading-snug italic">
              « {fallen.lastWords} »
            </blockquote>
            <p className="text-caption text-mute">{t('play.fallen.said')}</p>
          </>
        ) : (
          <>
            <p className="text-caption text-mute-soft">{t('play.fallen.wordsHint', { name })}</p>
            <textarea
              aria-label={t('play.fallen.words')}
              maxLength={WORDS_MAX}
              rows={3}
              value={words}
              onChange={(e) => setWords(e.target.value)}
              className="rounded-xl border border-line-strong bg-well px-3 py-2 text-body text-chalk focus:border-chalk focus:outline-none"
            />
            <CardButton
              title={t('play.fallen.say')}
              subtitle={t('play.fallen.saySub')}
              disabled={busy || !words.trim()}
              onClick={() => void send(() => sayLastWords(campaignId, words))}
            />
          </>
        )}
      </div>

      <div className="flex flex-col gap-2">
        <span className="type-label">{t('play.fallen.next')}</span>
        <p className="type-title text-[18px]">{t('play.fallen.stay')}</p>
        {CHOICES.map((c) => (
          <button
            key={c}
            type="button"
            disabled={busy}
            aria-pressed={fallen.next === c}
            onClick={() => void send(() => chooseNext(campaignId, c))}
            className={cn(
              'flex flex-col gap-1 rounded-[12px] border-[1.5px] p-3.5 text-left text-caption disabled:opacity-60',
              fallen.next === c
                ? 'border-ink bg-ivory text-ink-soft shadow-ivory-flat'
                : 'border-line bg-surface text-chalk-soft',
            )}
          >
            <b className={cn('type-title text-[17px]', fallen.next === c ? 'text-ink' : 'text-chalk')}>
              {t(`play.fallen.${c}`)}
            </b>
            <span>{t(`play.fallen.${c}Sub`, { name })}</span>
          </button>
        ))}
      </div>
      {failed && <p role="alert">{t('play.fallen.error')}</p>}
    </section>
  )
}
