import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { ArcadeCluster } from '@/components/game/ArcadeCluster'
import { BottomPanel } from '@/components/game/BottomPanel'
import { CardButton } from '@/components/game/CardButton'
import { CellBar } from '@/components/game/CellBar'
import { CONDITION_ICONS, ConditionBadge, type ConditionIcon } from '@/components/game/ConditionBadge'
import { GameCard, RarityPips, type CardState } from '@/components/game/GameCard'
import { Hearts } from '@/components/game/Hearts'
import { ITEM_SPRITES, type ItemSpriteName } from '@/components/game/itemSprites'
import { ItemSlot } from '@/components/game/ItemSlot'
import { RARITIES, type Rarity } from '@/components/game/rarity'
import { StatGem } from '@/components/game/StatGem'
import { STATS } from '@/components/game/stats'
import { StatusBanner, type BannerTone } from '@/components/game/StatusBanner'
import { ThreatClock } from '@/components/game/ThreatClock'
import { Toast, ToastStack, type ToastTone } from '@/components/game/Toast'
import { cn } from '@/lib/utils'

/**
 * Every game component of `components/game` with its states, as the
 * design boards show them (« Jauges et boutons », « Rareté des
 * compétences », « Objets et butin », « États des personnages »,
 * « Notifications »). Sample values are the boards' own.
 */
export function GameComponentsSection() {
  const { t } = useTranslation()
  // Remounts the one-shot events (hit, spent cells, stamp, toasts) to play them again.
  const [take, setTake] = useState(0)

  return (
    <section className="flex flex-col gap-5">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div className="flex flex-col gap-1">
          <p className="type-label">{t('reference.game.kicker')}</p>
          <h2 className="type-title text-heading-lg">{t('reference.game.title')}</h2>
        </div>
        <Button variant="outline" onClick={() => setTake((n) => n + 1)}>
          {t('reference.game.replay')}
        </Button>
      </header>
      <div key={take} className="flex flex-col gap-5">
        <div className="grid gap-5 lg:grid-cols-2">
          <GaugesSlab />
          <BarsSlab />
        </div>
        <div className="grid gap-5 lg:grid-cols-2">
          <ButtonsSlab />
          <ArcadeSlab />
        </div>
        <CardsSlab />
        <ItemsSlab />
        <ConditionsSlab />
        <NotificationsSlab />
      </div>
    </section>
  )
}

function Slab({ title, note, children, className }: { title: string; note?: string; children: ReactNode; className?: string }) {
  return (
    <section className={cn('surface-slab flex min-w-0 flex-col gap-4 p-5', className)}>
      <h3 className="type-label">{title}</h3>
      {children}
      {note && <p className="text-caption text-mute">{note}</p>}
    </section>
  )
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center gap-3">
      <span className="w-20 flex-none text-caption text-mute">{label}</span>
      {children}
    </div>
  )
}

// --- Gauges -----------------------------------------------------------

const GEM_VALUES = { hp: '18', atk: '+5', ac: '16', mag: '3', move: '6', init: '+1' } as const

function GaugesSlab() {
  const { t } = useTranslation()
  return (
    <Slab title={t('reference.game.gauges.title')} note={t('reference.game.gauges.note')}>
      <div className="flex flex-wrap items-end gap-4">
        {STATS.map((s) => (
          <StatGem key={s} stat={s} value={GEM_VALUES[s]} size={64} />
        ))}
      </div>
      <div className="flex flex-wrap items-end gap-3">
        {STATS.map((s) => (
          <StatGem key={s} stat={s} value={GEM_VALUES[s]} size={34} />
        ))}
        {STATS.map((s) => (
          <StatGem key={s} stat={s} size={22} animated={false} />
        ))}
      </div>
      <div className="flex flex-col gap-3 border-t border-line pt-4">
        <Row label={t('reference.game.gauges.rest')}>
          <Hearts hp={18} max={22} />
        </Row>
        <Row label={t('reference.game.gauges.hit')}>
          <Hearts hp={12} max={22} damage={6} />
        </Row>
        <Row label={t('reference.game.gauges.danger')}>
          <Hearts hp={5} max={22} />
        </Row>
        <Row label={t('reference.game.gauges.beyond')}>
          <Hearts hp={9} max={31} px={2} />
        </Row>
        <Row label={t('reference.game.gauges.tight')}>
          <Hearts hp={9} max={31} count={8} px={2} />
        </Row>
      </div>
    </Slab>
  )
}

function BarsSlab() {
  const { t } = useTranslation()
  return (
    <Slab title={t('reference.game.bars.title')} note={t('reference.game.bars.note')}>
      <Row label={t('reference.game.bars.slots')}>
        <CellBar stat="mag" value={3} max={4} label={t('reference.game.bars.slots')} width={120} height={16} />
      </Row>
      <Row label={t('reference.game.bars.move')}>
        <CellBar stat="move" value={4} max={6} label={t('reference.game.bars.move')} width={180} height={16} />
      </Row>
      <Row label={t('reference.game.bars.spent')}>
        <CellBar stat="move" value={2} max={6} spent={2} label={t('reference.game.bars.move')} width={180} height={16} />
      </Row>
      <Row label={t('reference.game.bars.low')}>
        <CellBar stat="mag" value={1} max={4} label={t('reference.game.bars.slots')} width={120} height={16} />
      </Row>
      <Row label={t('reference.game.bars.xp')}>
        <CellBar stat="init" value={8} max={10} label={t('reference.game.bars.xp')} width={200} height={10} />
      </Row>
      <Row label={t('reference.game.bars.twoRows')}>
        <CellBar stat="hp" value={12} max={22} spent={6} rows={2} height={9} width={240} />
      </Row>
      <div className="flex flex-wrap items-center gap-6 border-t border-line pt-4">
        <ThreatClock parts={6} filled={3} name={t('reference.game.bars.front')} />
        <ThreatClock parts={4} filled={1} size={72} name={t('reference.game.bars.front')} />
        <ThreatClock parts={6} filled={5} size={44} next={false} name={t('reference.game.bars.front')} />
        <p className="max-w-56 text-caption text-mute-soft">{t('reference.game.bars.clockNote')}</p>
      </div>
    </Slab>
  )
}

// --- Buttons ----------------------------------------------------------

function ButtonsSlab() {
  const { t } = useTranslation()
  return (
    <Slab title={t('reference.game.buttons.title')} note={t('reference.game.buttons.note')}>
      <div className="flex max-w-[280px] flex-col gap-4">
        <CardButton
          title={t('reference.game.buttons.attack')}
          subtitle={t('reference.game.buttons.attackSub')}
          gem={{ stat: 'atk', value: '+5' }}
          sheen
        />
        <CardButton
          title={t('reference.game.buttons.endTurn')}
          subtitle={t('reference.game.buttons.endTurnSub')}
          icon="hourglass"
          variant="dark"
        />
        <CardButton title={t('reference.game.buttons.sending')} subtitle={t('reference.game.buttons.sendingSub')} icon="send" pressed />
        <CardButton title={t('reference.game.buttons.unavailable')} icon="stop" variant="dark" disabled />
        <div className="grid grid-cols-2 gap-2.5">
          <CardButton title={t('reference.game.buttons.equip')} icon="sword" size="small" />
          <CardButton title={t('reference.game.buttons.give')} icon="arrow" size="small" variant="dark" />
        </div>
      </div>
    </Slab>
  )
}

function ArcadeSlab() {
  const { t } = useTranslation()
  return (
    <Slab title={t('reference.game.arcade.title')} note={t('reference.game.arcade.note')}>
      <div className="mx-auto w-full max-w-[366px]">
        <ArcadeCluster
          main={{ label: t('reference.game.arcade.attack'), icon: 'sword' }}
          left={{ label: t('reference.game.arcade.bag'), icon: 'flask' }}
          right={{ label: t('reference.game.arcade.endTurn'), icon: 'hourglass' }}
        />
      </div>
      <div className="mx-auto w-full max-w-[366px] border-t border-line pt-4">
        <ArcadeCluster
          main={{ label: t('reference.game.arcade.attack'), icon: 'sword' }}
          left={{ label: t('reference.game.arcade.bag'), icon: 'flask' }}
          right={{ label: t('reference.game.arcade.endTurn'), icon: 'hourglass' }}
          busy
        />
        <p className="mt-2 text-center text-caption text-mute">{t('reference.game.arcade.busy')}</p>
      </div>
    </Slab>
  )
}

// --- Cards ------------------------------------------------------------

const TIER_CARDS: Record<Rarity, { value: string; icon: 'sword' | 'skull' | 'spell' }> = {
  common: { value: '+5', icon: 'sword' },
  uncommon: { value: '+5', icon: 'sword' },
  rare: { value: '+6', icon: 'sword' },
  epic: { value: '+7', icon: 'sword' },
  legendary: { value: '+8', icon: 'skull' },
  divine: { value: '+10', icon: 'spell' },
}

const CARD_STATES: CardState[] = ['normal', 'active', 'greyed', 'back']

function CardsSlab() {
  const { t } = useTranslation()
  return (
    <Slab title={t('reference.game.cards.title')} note={t('reference.game.cards.note')}>
      <div className="flex flex-wrap items-end gap-6">
        <GameCard
          kind="action"
          title={t('reference.game.cards.strike')}
          text={t('reference.game.cards.strikeText')}
          value="+5"
          stat="atk"
        />
        <GameCard kind="clue" title={t('reference.game.cards.page')} text={t('reference.game.cards.pageText')} dealDelay={0.12} />
        <GameCard kind="scene" title={t('reference.game.cards.altar')} text={t('reference.game.cards.altarText')} dealDelay={0.24} />
        <GameCard kind="action" title={t('reference.game.cards.search')} text={t('reference.game.cards.searchText')} value="+4" icon="eye" dealDelay={0.36} />
      </div>
      <div className="grid grid-cols-2 gap-x-4 gap-y-8 border-t border-line pt-8 sm:grid-cols-3 lg:grid-cols-6">
        {RARITIES.map((r, i) => (
          <figure key={r} className="flex flex-col items-center gap-3">
            <GameCard
              kind="action"
              title={t(`reference.game.cards.tiers.${r}.title`)}
              text={t(`reference.game.cards.tiers.${r}.text`)}
              typeLabel={t('reference.game.cards.skill')}
              value={TIER_CARDS[r].value}
              stat="atk"
              icon={TIER_CARDS[r].icon}
              rarity={r}
              width={132}
              dealDelay={i * 0.12}
            />
            <figcaption className="flex flex-col items-center gap-1.5 text-center">
              <RarityPips rarity={r} size={8} className="[--pip-color:var(--color-chalk)] [--pip-rim:transparent]" />
              <span className="type-title text-body">{t(`reference.materials.tiers.${r}.name`)}</span>
              <span className="text-caption text-mute">{t(`reference.materials.tiers.${r}.how`)}</span>
            </figcaption>
          </figure>
        ))}
      </div>
      <div className="flex flex-wrap items-end gap-6 border-t border-line pt-8">
        {CARD_STATES.map((state) => (
          <figure key={state} className="flex flex-col items-center gap-3">
            <GameCard
              kind="action"
              title={t('reference.game.cards.strike')}
              text={t('reference.game.cards.strikeText')}
              value="+5"
              stat="atk"
              state={state}
              width={100}
              deal={false}
            />
            <figcaption className="text-caption text-mute">{t(`reference.game.cards.states.${state}`)}</figcaption>
          </figure>
        ))}
      </div>
    </Slab>
  )
}

// --- Items ------------------------------------------------------------

const TIER_ITEMS: Record<Rarity, ItemSpriteName> = {
  common: 'potion',
  uncommon: 'purse',
  rare: 'shield',
  epic: 'ring',
  legendary: 'axe',
  divine: 'orb',
}

/** Borin's bag on the board: sprite, rarity, quantity. */
const BAG: ({ sprite: ItemSpriteName; rarity: Rarity; quantity?: number; selected?: boolean } | null)[] = [
  { sprite: 'axe', rarity: 'legendary', selected: true },
  { sprite: 'potion', rarity: 'common', quantity: 3 },
  { sprite: 'shield', rarity: 'rare' },
  { sprite: 'scroll', rarity: 'uncommon', quantity: 2 },
  { sprite: 'purse', rarity: 'common', quantity: 48 },
  { sprite: 'ring', rarity: 'epic' },
  { sprite: 'sword', rarity: 'uncommon' },
  null,
]

function ItemsSlab() {
  const { t } = useTranslation()
  return (
    <Slab title={t('reference.game.items.title')} note={t('reference.game.items.note')}>
      <div className="grid grid-cols-3 gap-4 sm:grid-cols-6">
        {RARITIES.map((r) => (
          <figure key={r} className="flex flex-col items-center gap-2 text-center">
            <ItemSlot
              sprite={ITEM_SPRITES[TIER_ITEMS[r]]}
              name={t(`reference.game.items.names.${TIER_ITEMS[r]}`)}
              rarity={r}
              size={88}
            />
            <figcaption className="type-title text-caption">{t(`reference.game.items.names.${TIER_ITEMS[r]}`)}</figcaption>
          </figure>
        ))}
      </div>
      <div className="flex flex-wrap gap-2 border-t border-line pt-4">
        {BAG.map((slot, i) =>
          slot ? (
            <ItemSlot
              key={i}
              sprite={ITEM_SPRITES[slot.sprite]}
              name={t(`reference.game.items.names.${slot.sprite}`)}
              rarity={slot.rarity}
              quantity={slot.quantity}
              selected={slot.selected}
              size={64}
            />
          ) : (
            <ItemSlot key={i} size={64} />
          ),
        )}
      </div>
      <div className="flex flex-wrap items-end gap-4 border-t border-line pt-4">
        <ItemSlot sprite={ITEM_SPRITES.chest} name={t('reference.game.items.names.chest')} framed={false} size={84} />
        <ItemSlot sprite={ITEM_SPRITES.crown} name={t('reference.game.items.names.crown')} rarity="legendary" size={48} pips={false} />
        <ItemSlot sprite={ITEM_SPRITES.scroll} name={t('reference.game.items.names.scroll')} size={24} pips={false} />
      </div>
    </Slab>
  )
}

// --- Conditions -------------------------------------------------------

const CONDITIONS: Record<ConditionIcon, { kind: 'boon' | 'bane'; turns?: number }> = {
  poison: { kind: 'bane', turns: 3 },
  stun: { kind: 'bane', turns: 1 },
  sleep: { kind: 'bane' },
  invisible: { kind: 'boon', turns: 2 },
  fire: { kind: 'bane', turns: 2 },
  restrained: { kind: 'bane' },
  fear: { kind: 'bane', turns: 1 },
  blessed: { kind: 'boon', turns: 10 },
}

function ConditionsSlab() {
  const { t } = useTranslation()
  return (
    <Slab title={t('reference.game.conditions.title')} note={t('reference.game.conditions.note')}>
      <ul className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        {CONDITION_ICONS.map((icon) => (
          <li key={icon} className="flex items-center gap-3">
            <ConditionBadge
              icon={icon}
              name={t(`reference.game.conditions.names.${icon}`)}
              kind={CONDITIONS[icon].kind}
              turns={CONDITIONS[icon].turns}
              size={30}
            />
            <span className="type-title text-body">{t(`reference.game.conditions.names.${icon}`)}</span>
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap items-center gap-3 border-t border-line pt-4">
        <span className="text-caption text-mute">{t('reference.game.conditions.fresh')}</span>
        <ConditionBadge icon="poison" name={t('reference.game.conditions.names.poison')} kind="bane" turns={3} size={30} fresh />
        <ConditionBadge icon="blessed" name={t('reference.game.conditions.names.blessed')} kind="boon" turns={1} size={24} />
        <ConditionBadge icon="stun" name={t('reference.game.conditions.names.stun')} kind="bane" size={18} />
      </div>
    </Slab>
  )
}

// --- Notifications ----------------------------------------------------

const BANNERS: BannerTone[] = ['wait', 'listen', 'you', 'turn', 'star', 'hurt', 'warn']

function NotificationsSlab() {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [toasts, setToasts] = useState<ToastTone[]>(['good', 'bad', 'info'])

  return (
    <div className="grid gap-5 lg:grid-cols-2">
      <Slab title={t('reference.game.banners.title')} note={t('reference.game.banners.note')}>
        <div className="flex max-w-[366px] flex-col gap-3">
          {BANNERS.map((tone) => (
            <StatusBanner key={tone} tone={tone}>
              {t(`reference.game.banners.${tone}`)}
            </StatusBanner>
          ))}
        </div>
      </Slab>
      <Slab title={t('reference.game.toasts.title')} note={t('reference.game.toasts.note')}>
        <ToastStack className="max-w-[366px]">
          {toasts.map((tone) => (
            <Toast
              key={tone}
              tone={tone}
              kicker={t(`reference.game.toasts.${tone}.kicker`)}
              media={<ToastMedia tone={tone} />}
              onDismiss={() => setToasts((list) => list.filter((x) => x !== tone))}
            >
              {t(`reference.game.toasts.${tone}.message`)}
            </Toast>
          ))}
        </ToastStack>
        <div className="flex flex-wrap gap-3 border-t border-line pt-4">
          <Button variant="outline" onClick={() => setToasts(['good', 'bad', 'info'])}>
            {t('reference.game.toasts.again')}
          </Button>
          <Button variant="outline" onClick={() => setOpen(true)}>
            {t('reference.game.panel.open')}
          </Button>
        </div>
        <BottomPanel open={open} onOpenChange={setOpen} title={t('reference.game.panel.title')}>
          <p className="text-caption text-mute-soft">{t('reference.game.panel.note')}</p>
          <div className="flex flex-wrap gap-2">
            {BAG.map((slot, i) =>
              slot ? (
                <ItemSlot
                  key={i}
                  sprite={ITEM_SPRITES[slot.sprite]}
                  name={t(`reference.game.items.names.${slot.sprite}`)}
                  rarity={slot.rarity}
                  quantity={slot.quantity}
                  size={56}
                  pips={false}
                />
              ) : (
                <ItemSlot key={i} size={56} />
              ),
            )}
          </div>
          <CardButton title={t('reference.game.panel.back')} variant="dark" onClick={() => setOpen(false)} />
        </BottomPanel>
      </Slab>
    </div>
  )
}

function ToastMedia({ tone }: { tone: ToastTone }) {
  const { t } = useTranslation()
  if (tone === 'good') {
    return <ItemSlot sprite={ITEM_SPRITES.shield} name={t('reference.game.items.names.shield')} rarity="rare" size={48} pips={false} />
  }
  if (tone === 'bad') {
    return (
      <span className="block px-[9px]">
        <ConditionBadge icon="poison" name={t('reference.game.conditions.names.poison')} kind="bane" turns={3} size={30} fresh />
      </span>
    )
  }
  return <ItemSlot sprite={ITEM_SPRITES.scroll} name={t('reference.game.items.names.scroll')} size={48} pips={false} />
}
