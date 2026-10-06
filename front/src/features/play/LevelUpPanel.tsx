import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { spendUpgrade, takeLevel, type CharacterView, type LevelTaken, type PlayView } from '@/lib/play'
import { ActionCard, signed } from './creator/RuleSteps'

/**
 * Levelling up (planche « Entre deux », moment 2): one real choice per
 * level, the die or its average, rolled by the server; the cards the
 * level unlocks arrive by themselves. Upgrade points go to an ability,
 * one tap each. Shown while something is left to choose.
 */
export function LevelUpPanel({
  campaignId,
  play,
  onChanged,
}: {
  campaignId: string
  play: PlayView
  onChanged: (character: CharacterView) => void
}) {
  const { t } = useTranslation()
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)
  const [taken, setTaken] = useState<LevelTaken | null>(null)
  const level = play.levelsToChoose[0]
  const hp = play.levelHitPoints
  const shownLevel = level ?? taken?.level
  const newCards = shownLevel === undefined ? [] : play.cards.filter((c) => c.level === shownLevel)

  if (level === undefined && taken === null && play.upgradePoints === 0) return null

  async function run(task: () => Promise<void>) {
    setBusy(true)
    setFailed(false)
    try {
      await task()
    } catch {
      setFailed(true)
    } finally {
      setBusy(false)
    }
  }

  function take(choice: 'roll' | 'average') {
    if (level === undefined) return
    void run(async () => {
      const r = await takeLevel(campaignId, level, choice)
      setTaken(r.taken)
      onChanged(r.character)
    })
  }

  function upgrade(ability: string) {
    void run(async () => onChanged(await spendUpgrade(campaignId, ability)))
  }

  return (
    <section aria-label={t('play.level.title')} className="flex flex-col gap-3 rounded-[14px] border-2 border-ink bg-surface p-3.5">
      {shownLevel !== undefined && (
        <h2 className="type-title text-[20px]">{t('play.level.reached', { level: shownLevel })}</h2>
      )}
      {level !== undefined && hp && (
        <div className="flex flex-col gap-2">
          <span className="type-label">{t('play.level.hitPoints')}</span>
          <div className="grid grid-cols-2 gap-2">
            <Button variant="outline" disabled={busy} onClick={() => take('roll')} className="h-auto flex-col py-3">
              <b>{t('play.level.roll')}</b>
              <span className="text-caption">{t('play.level.rollHint', { dice: hp.dice, bonus: signed(hp.bonus) })}</span>
            </Button>
            <Button variant="outline" disabled={busy} onClick={() => take('average')} className="h-auto flex-col py-3">
              <b>{t('play.level.average')}</b>
              <span className="text-[22px] font-bold">{signed(Math.max(1, hp.average + hp.bonus))}</span>
              <span className="text-caption">{t('play.level.averageHint')}</span>
            </Button>
          </div>
        </div>
      )}
      {taken && (
        <p role="status" className="text-body">
          {taken.faces
            ? t('play.level.rolled', { faces: taken.faces.join(' + '), gain: taken.maxAfter - taken.maxBefore, max: taken.maxAfter })
            : t('play.level.averaged', { gain: taken.maxAfter - taken.maxBefore, max: taken.maxAfter })}
        </p>
      )}
      {newCards.length > 0 && (
        <div className="flex flex-col gap-2">
          <span className="type-label">{t('play.level.newCard', { level: shownLevel })}</span>
          <div className="flex gap-2 overflow-x-auto">
            {newCards.map((c) => (
              <ActionCard key={c.id} card={c} width={96} level={play.level} />
            ))}
          </div>
        </div>
      )}
      {play.upgradePoints > 0 && (
        <div className="flex flex-col gap-2">
          <span className="type-label">{t('play.level.upgrade', { count: play.upgradePoints })}</span>
          <ul className="grid grid-cols-3 gap-1.5">
            {play.abilities.map((a) => (
              <li key={a.id}>
                <Button
                  variant="outline"
                  disabled={busy}
                  onClick={() => upgrade(a.id)}
                  aria-label={t('play.level.raise', { ability: a.name })}
                  className="h-auto w-full flex-col py-1.5"
                >
                  <span className="text-[10px] tracking-[0.12em] uppercase">{a.name}</span>
                  <b>
                    {a.score} → {a.score + 1}
                  </b>
                </Button>
              </li>
            ))}
          </ul>
        </div>
      )}
      {failed && <p role="alert">{t('play.error')}</p>}
    </section>
  )
}
