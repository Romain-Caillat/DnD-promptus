import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { CellBar } from '@/components/game/CellBar'
import type { BetweenView } from '@/lib/between'

/**
 * Between two sessions, on the Game tab (player/play-between-sessions,
 * planche « Entre deux », moments 1 and 3): how the last evening ended
 * for the player — the XP it brought and the level reached, what they
 * got — the way to level up, then « Précédemment… » once the GM
 * published it, what the table learnt and what stays open, and the way
 * to the chronicle and to the sheet. Everything comes from the server's
 * projection (`GET /api/play/…/between`).
 */
export function EveningEnd({
  between,
  onLevelUp,
  onChronicle,
  onSheet,
}: {
  between: BetweenView
  onLevelUp: () => void
  onChronicle: () => void
  onSheet: () => void
}) {
  const { t } = useTranslation()
  const { last, levelUp } = between
  if (!last) return null
  const mine = last.mine
  const played =
    last.minutes === null
      ? null
      : last.minutes >= 60
        ? t('between.playedHours', {
            hours: Math.floor(last.minutes / 60),
            minutes: String(last.minutes % 60).padStart(2, '0'),
          })
        : t('between.playedMinutes', { minutes: last.minutes })
  const levelled = mine !== null && mine.level > mine.levelBefore

  return (
    <section className="flex flex-col gap-3" aria-label={t('between.endTitle', { number: last.number })}>
      <div className="flex flex-col gap-1">
        <span className="type-label">
          {[t('between.endTitle', { number: last.number }), played].filter(Boolean).join(' · ')}
        </span>
        {last.title && <h2 className="type-title text-[24px] leading-tight">{last.title}</h2>}
      </div>

      {mine && (
        <div className="flex flex-col gap-1.5 rounded-xl border border-line bg-surface px-3.5 py-3">
          <span className="flex justify-between text-caption text-mute-soft">
            <b className="text-chalk">{t('between.xp')}</b>
            <span>
              {mine.xpGained > 0 ? t('between.xpGained', { count: mine.xpGained }) : t('between.noXp')}
              {levelled && ` · ${t('between.levelReached', { level: mine.level })}`}
            </span>
          </span>
          <CellBar stat="init" value={mine.xpBar} max={mine.xpBarMax} height={10} label={t('between.xp')} />
          <span className="text-caption text-mute">{t('between.xpTotal', { total: mine.totalXp })}</span>
        </div>
      )}

      {mine && mine.got.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <h3 className="type-label">{t('between.got')}</h3>
          <ul className="flex flex-col gap-1.5">
            {mine.got.map((line, i) => (
              <li key={i} className="pixel-field px-3 py-2 text-body">
                {line}
              </li>
            ))}
          </ul>
        </div>
      )}

      {levelUp && (
        <CardButton
          title={
            levelled ? t('between.levelUp', { level: levelUp.level }) : t('between.placePoints', { count: levelUp.points })
          }
          subtitle={t('between.levelUpHint')}
          onClick={onLevelUp}
        />
      )}

      {last.previously ? (
        <div className="surface-slab flex flex-col gap-2 p-3.5">
          <span className="type-label">{t('between.previouslyOf', { number: last.number })}</span>
          <span className="type-title text-[24px]">{t('between.previously')}</span>
          <p className="type-narration text-[18px] leading-snug">{last.previously}</p>
        </div>
      ) : (
        <p className="text-caption text-mute">{t('between.waitingRecap')}</p>
      )}

      {last.learnt.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <h3 className="type-label">{t('between.learnt')}</h3>
          <ul className="flex flex-col gap-1.5">
            {last.learnt.map((line, i) => (
              <li key={i} className="rounded-button bg-ivory px-3 py-2 text-body text-ink shadow-ivory-flat">
                {line}
              </li>
            ))}
          </ul>
        </div>
      )}

      {between.openThreads.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <h3 className="type-label">{t('between.open')}</h3>
          <ul className="flex flex-col gap-1.5">
            {between.openThreads.map((line, i) => (
              <li key={i} className="pixel-field px-3 py-2 text-body">
                {line}
              </li>
            ))}
          </ul>
        </div>
      )}

      <CardButton
        variant="dark"
        title={t('between.readChronicle')}
        subtitle={t('between.readChronicleHint', { count: between.chronicle.length })}
        onClick={onChronicle}
      />
      {mine && (
        <CardButton variant="dark" title={t('between.seeSheet')} subtitle={t('between.seeSheetHint')} onClick={onSheet} />
      )}
    </section>
  )
}
