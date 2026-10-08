import { useTranslation } from 'react-i18next'
import { StatGem } from '@/components/game/StatGem'
import type { Stat } from '@/components/game/stats'
import { Sprite } from '@/features/sprites/Sprite'
import type { CharacterLook } from '@/features/sprites/look'
import type { CharacterSheet, SheetStats } from '@/lib/play'
import { signed } from './RuleSteps'

/**
 * What goes to the GM (moment 8): the character drawn, who they are, the
 * numbers the server computed for them, the point over the rules if
 * any, and the start of their story.
 */
export function CharacterSummary({
  sheet,
  look,
  subtitle,
  stats,
  over,
  gmName,
}: {
  sheet: CharacterSheet
  look: CharacterLook
  /** « Bretteur · niveau 1 ». */
  subtitle: string
  stats: SheetStats | null
  /** Points over the class's spread. */
  over: number
  gmName: string
}) {
  const { t } = useTranslation()
  const name = sheet.name || t('play.unnamed')
  const attack = stats?.cards
    .filter((c) => c.level <= 1 && c.attackBonus !== null)
    .reduce<number | null>((best, c) => Math.max(best ?? -99, c.attackBonus!), null)
  const gems: { stat: Stat; value: string; label: string }[] = stats
    ? [
        { stat: 'hp', value: String(stats.hitPoints), label: t('creator.review.hp') },
        { stat: 'ac', value: String(stats.armorClass), label: t('creator.review.ac') },
        ...(attack != null ? [{ stat: 'atk' as const, value: signed(attack), label: t('creator.review.atk') }] : []),
        { stat: 'init', value: signed(stats.initiative), label: t('creator.review.init') },
      ]
    : []
  const story = sheet.backstory?.text || [sheet.backstory?.origin, sheet.backstory?.loss, sheet.backstory?.quest].filter(Boolean).join(' · ')

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-end gap-3.5 rounded-none border border-line bg-surface p-3.5">
        <Sprite look={look} scale={4} label={name} />
        <div className="flex flex-col gap-1">
          <span className="type-title text-[24px]">{name}</span>
          <span className="text-caption text-mute-soft">{subtitle}</span>
        </div>
      </div>
      {gems.length > 0 && (
        <div className="flex flex-wrap items-end gap-3.5">
          {gems.map((g) => (
            <span key={g.stat} className="flex flex-col items-center gap-1 text-[10px] font-semibold tracking-[0.12em] text-mute uppercase">
              <StatGem stat={g.stat} value={g.value} size={40} animated={false} />
              {g.label}
            </span>
          ))}
        </div>
      )}
      {over > 0 && (
        <div className="cr-warn" role="note">
          <b>{t('creator.review.overTitle', { count: over })}</b>
          {t('creator.review.over', { count: over, gm: gmName })}
        </div>
      )}
      {story && (
        <p className="line-clamp-3 pixel-field px-3.5 py-3 text-caption text-chalk-soft">
          {story}
        </p>
      )}
    </div>
  )
}
