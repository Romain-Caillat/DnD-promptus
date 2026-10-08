import { useTranslation } from 'react-i18next'
import type { ActionCardView } from '@/lib/play'
import { signed } from '../creator/RuleSteps'

/**
 * What a touched card does (planche « Entre deux », moment 5): its kind
 * and the level it opens at, its text, and the numbers the server
 * computed for this character — to hit, damage or healing, recharge,
 * reach. Read-only: outside play the sheet changes only through the GM.
 */
export function CardDetail({ card, onClose }: { card: ActionCardView; onClose: () => void }) {
  const { t } = useTranslation()
  const facts = [
    card.attackBonus !== null && t('between.card.attack', { bonus: signed(card.attackBonus) }),
    card.damage && t('between.card.damage', { amount: card.damage }),
    card.heal && t('between.card.heal', { amount: card.heal }),
    card.cooldown > 0 ? t('between.card.cooldown', { count: card.cooldown }) : t('between.card.noCooldown'),
    card.range > 0 && t('between.card.range', { count: card.range }),
  ].filter((x): x is string => Boolean(x))
  return (
    <div
      role="region"
      aria-label={card.name}
      className="flex flex-col gap-1.5 rounded-xl border-[1.5px] border-chalk bg-surface p-3"
    >
      <span className="flex items-start justify-between gap-2">
        <b className="type-title text-[16px]">{card.name}</b>
        <button type="button" className="text-caption text-mute-soft underline" onClick={onClose}>
          {t('between.card.close')}
        </button>
      </span>
      <span className="text-caption text-mute">{t('between.card.kind', { kind: card.kind, level: card.level })}</span>
      {card.description && <p className="text-body text-chalk-soft">{card.description}</p>}
      <ul className="flex flex-wrap gap-x-3 gap-y-1 text-caption text-chalk">
        {facts.map((f) => (
          <li key={f}>{f}</li>
        ))}
      </ul>
    </div>
  )
}
