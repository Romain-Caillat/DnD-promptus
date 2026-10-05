import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { CARD_ICONS, LineIcon, type CardIcon } from './icons'
import { RARITY_MATERIAL, rarityRank, type Rarity } from './rarity'
import { StatGem } from './StatGem'
import type { Stat } from './stats'

export type CardKind = 'action' | 'clue' | 'scene'
export type CardState = 'normal' | 'active' | 'greyed' | 'back'

const KIND_ICON: Record<CardKind, CardIcon> = { action: 'sword', clue: 'scroll', scene: 'door' }

// Where the sparks of a legendary or divine card twinkle, in % of the card.
const SPARKS = [
  { x: -4, y: 8, d: 0 },
  { x: 92, y: 20, d: 0.6 },
  { x: 88, y: 84, d: 1.2 },
  { x: -2, y: 70, d: 1.7 },
]

/**
 * An ivory playing card on the black table (board « Composant — carte à
 * jouer »): an action, a clue or a scene. The gem in the corner carries
 * the stat it rolls. A skill card has a rarity, read from its material
 * and from 1 to 6 diamonds, never from a hue (board « Rareté des
 * compétences »). Cards are dealt in on mount; the hover lift and the
 * sheen of rare materials are ambient.
 */
export function GameCard({
  kind,
  title,
  text,
  typeLabel,
  value = '',
  stat,
  icon,
  rarity,
  state = 'normal',
  width = 150,
  deal = true,
  dealDelay = 0,
  className,
}: {
  kind: CardKind
  title: string
  /** One line of rule or flavour under the art. */
  text?: string
  /** The line above the text; the kind's name by default (« Indice »). */
  typeLabel?: string
  /** The corner value (« +5 », « d10 »); no gem when empty. */
  value?: string
  /** The stat behind the value; a plain black disc when none. */
  stat?: Stat
  icon?: CardIcon
  rarity?: Rarity
  /** `active`: picked in the hand; `greyed`: not playable now; `back`: face down. */
  state?: CardState
  width?: number
  deal?: boolean
  dealDelay?: number
  className?: string
}) {
  const { t } = useTranslation()
  const height = Math.round(width * 1.4)
  const fontSize = Math.round((width / 150) * 14 * 10) / 10
  const gemSize = Math.round(width * 0.34)
  const face = state !== 'back'
  const sparkle = face && (rarity === 'legendary' || rarity === 'divine')

  return (
    <div
      className={cn('gk-card-wrap', deal && 'animate-deal', className)}
      data-state={state}
      data-rarity={rarity ?? 'none'}
      style={deal ? { animationDelay: `${dealDelay}s` } : undefined}
    >
      <div className="gk-card-float">
        {face && rarity === 'divine' && <div className="gk-rays" aria-hidden />}
        <div
          className={cn('gk-card', face && RARITY_MATERIAL[rarity ?? 'common'])}
          style={{ width, height, fontSize }}
        >
          {face ? (
            <div className="gk-card-in">
              <div
                className="gk-card-title"
                style={{ paddingLeft: value ? Math.round(width * 0.27) : Math.round(width * 0.05) }}
              >
                {title}
              </div>
              <div className="gk-card-art">
                <LineIcon paths={CARD_ICONS[icon ?? KIND_ICON[kind]]} size={Math.round(width * 0.32)} strokeWidth={1.6} />
              </div>
              <div className="gk-card-type">{typeLabel ?? t(`game.card.kinds.${kind}`)}</div>
              <div className="gk-card-text">{text}</div>
            </div>
          ) : (
            <div className="gk-card-in">
              <span className="gk-card-emblem" role="img" aria-label={t('game.card.back')}>
                {t('game.card.emblem')}
              </span>
            </div>
          )}
          {face && value && stat && (
            <div className="gk-card-gem" style={{ width: gemSize, height: gemSize }}>
              <StatGem stat={stat} value={value} size={gemSize} />
            </div>
          )}
          {face && value && !stat && <div className="gk-card-disc">{value}</div>}
          {face && rarity && <RarityPips rarity={rarity} className="gk-card-pips" />}
        </div>
        {sparkle &&
          SPARKS.map((s) => (
            <span
              key={s.x}
              aria-hidden
              className="gk-spark"
              style={{ left: `${s.x}%`, top: `${s.y}%`, animationDelay: `${s.d}s`, fontSize, width: '.7em', height: '.7em' }}
            />
          ))}
      </div>
    </div>
  )
}

/**
 * The rarity of a card or an item without colour: 1 to 6 diamonds, named
 * for screen readers (« Rareté : épique »).
 */
export function RarityPips({
  rarity,
  size,
  className,
}: {
  rarity: Rarity
  /** Side of one diamond in pixels; follows the font size when omitted. */
  size?: number
  className?: string
}) {
  const { t } = useTranslation()
  const n = rarityRank(rarity)
  return (
    <span
      role="img"
      aria-label={t('game.rarity.label', { name: t(`game.rarity.${rarity}`) })}
      className={cn('gk-pips', className)}
      style={size ? { gap: Math.round(size * 0.7) } : undefined}
    >
      {Array.from({ length: n }, (_, i) => (
        <i key={i} style={size ? { width: size, height: size } : undefined} />
      ))}
    </span>
  )
}
