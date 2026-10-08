import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { levelUp, type CharacterView, type LevelUp, type LevelUpChoice } from '@/lib/play'
import { cn } from '@/lib/utils'
import { ActionCard } from './creator/RuleSteps'

/**
 * engine/level-up on the phone (planche « Entre deux », moment 2): the
 * new level, then one real choice per level when the rules add hit
 * points — the die the server rolls, or the average —, then the class
 * cards the level unlocks, and « C'est noté ». A level's hit points are
 * taken once, never rerolled: the server refuses a second try.
 *
 * Self-contained so the end-of-evening screen (`player/play-between-sessions`)
 * can mount it as is.
 */
export function LevelUpCard({
  campaignId,
  name,
  up,
  maxHitPoints,
  onChanged,
}: {
  campaignId: string
  name: string
  up: LevelUp
  maxHitPoints: number
  onChanged: (character: CharacterView) => void
}) {
  const { t } = useTranslation()
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)
  const hp = up.hitPoints
  const due = hp?.due ?? []

  async function send(choice: LevelUpChoice) {
    setBusy(true)
    setFailed(false)
    try {
      onChanged(await levelUp(campaignId, choice))
    } catch {
      setFailed(true)
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="flex flex-col gap-3" aria-label={t('play.levelUp.title', { level: up.to })}>
      <div className="flex flex-col items-center gap-1 rounded-2xl bg-ivory p-4 text-center text-ink shadow-ivory-flat">
        <span className="type-label text-ink-soft">{t('play.levelUp.title', { level: up.to })}</span>
        <span className="text-body">
          {t('play.levelUp.lead', { name, level: up.to, from: up.from, count: up.to - up.from })}
        </span>
      </div>

      {hp && due.length > 0 && (
        <div className="flex flex-col gap-2">
          <span className="type-label">{t('play.levelUp.hitPoints', { level: due[0] })}</span>
          <div className="grid grid-cols-2 gap-2">
            <ChoiceButton
              title={t('play.levelUp.roll')}
              sub={
                hp.ability
                  ? t('play.levelUp.rollSub', { dice: hp.dice, ability: hp.ability })
                  : t('play.levelUp.rollSubPlain', { dice: hp.dice })
              }
              disabled={busy}
              onClick={() => void send({ kind: 'hitPoints', level: due[0], method: 'roll' })}
            />
            <ChoiceButton
              title={t('play.levelUp.average')}
              sub={t('play.levelUp.averageSub', { amount: Math.max(1, hp.average + hp.modifier) })}
              disabled={busy}
              onClick={() => void send({ kind: 'hitPoints', level: due[0], method: 'average' })}
            />
          </div>
        </div>
      )}
      {up.gains.length > 0 && (
        <ul className="flex flex-col gap-1 text-caption text-chalk-soft">
          {up.gains.map((g) => (
            <li key={g.level}>
              {g.method === 'roll'
                ? t('play.levelUp.takenRoll', { level: g.level, faces: g.faces.join(', '), amount: g.amount })
                : t('play.levelUp.taken', { level: g.level, amount: g.amount })}
            </li>
          ))}
        </ul>
      )}

      <span className="type-label">{t('play.levelUp.cards')}</span>
      {up.cards.length > 0 ? (
        <div className="-mx-4 flex gap-2 overflow-x-auto px-4 pb-1">
          {up.cards.map((c) => (
            <ActionCard key={c.id} card={c} width={110} level={up.to} />
          ))}
        </div>
      ) : (
        <p className="text-caption text-mute">{t('play.levelUp.noCard')}</p>
      )}

      <CardButton
        title={t('play.levelUp.seen')}
        subtitle={t('play.levelUp.seenSub', { name, hp: maxHitPoints })}
        disabled={busy || due.length > 0}
        onClick={() => void send({ kind: 'seen' })}
      />
      {failed && <p role="alert">{t('play.levelUp.error')}</p>}
    </section>
  )
}

function ChoiceButton({
  title,
  sub,
  disabled,
  onClick,
}: {
  title: string
  sub: string
  disabled: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={cn(
        'pixel-choice flex flex-col items-center gap-1 py-3 pr-2 pb-3.5 text-center text-caption text-chalk-soft',
      )}
    >
      <b className="type-title text-[16px] text-chalk">{title}</b>
      <span>{sub}</span>
    </button>
  )
}
