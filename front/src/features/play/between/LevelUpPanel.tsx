import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { Sprite } from '@/features/sprites/Sprite'
import { ApiError } from '@/lib/api'
import { spendUpgrade } from '@/lib/between'
import type { CharacterView, PlayView } from '@/lib/play'
import { cn } from '@/lib/utils'
import { ActionCard, signed } from '../creator/RuleSteps'
import { useBetween } from './useBetween'

/**
 * Levelling up (planche « Entre deux », moment 2): the one real choice
 * the two witness worlds make at a new level — where the upgrade points
 * the XP gave go, one point at a time — and the cards the new level
 * opened, which arrive in the hand on their own. The server spends the
 * point (`POST /api/play/…/character/upgrade`) and refuses while a
 * session is live; the scores and modifiers shown are the server's.
 * Hit points by die or average belong to rule systems with hit dice
 * (`engine/level-up`); both witness worlds keep 10.
 */
export function LevelUpPanel({
  campaignId,
  character,
  play,
  refreshKey,
  onChanged,
}: {
  campaignId: string
  character: CharacterView
  play: PlayView
  refreshKey: number
  onChanged: (character: CharacterView) => void
}) {
  const { t } = useTranslation()
  // The cards opened come from the between view; the points from `play`.
  const between = useBetween(campaignId, refreshKey + play.upgradePoints)
  const [picked, setPicked] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const name = character.sheet.name || t('play.unnamed')
  const live = between.kind === 'ready' && between.between.open?.status === 'live'
  const newCards = between.kind === 'ready' ? (between.between.levelUp?.newCards ?? []) : []
  const ability = play.abilities.find((a) => a.id === picked) ?? null

  async function spend() {
    if (!ability) return
    setBusy(true)
    setError(null)
    try {
      onChanged(await spendUpgrade(campaignId, ability.id))
      setPicked(null)
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'UNEXPECTED')
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="flex flex-col gap-3" aria-label={t('between.level.title', { name, level: play.level })}>
      <div className="flex flex-col items-center gap-2 rounded-2xl bg-ivory p-4 text-center text-ink shadow-ivory-flat">
        {character.sheet.look && <Sprite look={character.sheet.look} scale={5} label={name} />}
        <span className="type-title text-[22px]">{t('between.level.title', { name, level: play.level })}</span>
        <span className="text-caption text-ink-soft">{t('between.level.points', { count: play.upgradePoints })}</span>
      </div>

      {live ? (
        <p className="text-body text-chalk-soft">{t('between.level.live')}</p>
      ) : (
        <>
          <h3 className="type-label">{t('between.level.pick')}</h3>
          <div className="grid grid-cols-3 gap-1.5" role="radiogroup" aria-label={t('between.level.pick')}>
            {play.abilities.map((a) => (
              <button
                key={a.id}
                type="button"
                role="radio"
                aria-checked={picked === a.id}
                className={cn(
                  'flex flex-col items-center rounded-xl border-[1.5px] px-2 py-2',
                  picked === a.id ? 'border-ink bg-ivory text-ink shadow-ivory-flat' : 'border-line bg-well',
                )}
                onClick={() => setPicked(a.id)}
              >
                <span className="text-[10px] font-semibold tracking-[0.12em] uppercase">{a.name}</span>
                <b className="type-title text-[20px]">{a.score}</b>
                <span className="text-caption">{signed(a.modifier)}</span>
              </button>
            ))}
          </div>
          {ability && (
            <CardButton
              title={t('between.level.confirm', { ability: ability.name })}
              subtitle={t('between.level.confirmHint', { from: ability.score, to: ability.score + 1 })}
              disabled={busy}
              onClick={() => void spend()}
            />
          )}
        </>
      )}
      {error && (
        <p role="alert" className="rounded-button border border-stat-atk px-3 py-2 text-body">
          {t(`between.errors.${error}`, { defaultValue: t('between.errors.UNEXPECTED') })}
        </p>
      )}

      {newCards.length > 0 && (
        <div className="flex flex-col gap-2">
          <h3 className="type-label">{t('between.level.newCards')}</h3>
          <div className="-mx-4 flex gap-2 overflow-x-auto px-4 pb-2">
            {newCards.map((c) => (
              <div key={c.id} className="flex-none">
                <ActionCard card={c} width={96} level={play.level} />
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  )
}
