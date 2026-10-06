import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CellBar } from '@/components/game/CellBar'
import { Hearts } from '@/components/game/Hearts'
import { StatGem } from '@/components/game/StatGem'
import { Button } from '@/components/ui/button'
import { Sprite } from '@/features/sprites/Sprite'
import { equipItem, type BagItem, type CharacterView, type PlayView } from '@/lib/play'
import { ActionCard, signed } from './creator/RuleSteps'
import { LevelUpPanel } from './LevelUpPanel'

/**
 * The Character tab once in play (planche « Jouer », onglet Personnage):
 * who they are and their level, their hearts, what they are (armour,
 * initiative, abilities), the cards they know — the locked ones greyed
 * with their level —, their purse, what they carry and their bag. Every
 * number comes from the server (`projection::PlayView`), which the GM
 * changes live; the one choice here is what the character carries.
 */
export function CharacterTab({
  campaignId,
  character,
  play,
  onChanged,
}: {
  campaignId: string
  character: CharacterView
  play: PlayView
  /** The server's answer to a change made here. */
  onChanged: (character: CharacterView) => void
}) {
  const { t } = useTranslation()
  const [busy, setBusy] = useState<string | null>(null)
  const [failed, setFailed] = useState(false)
  const name = character.sheet.name || t('play.unnamed')
  const attack = play.cards
    .filter((c) => c.level <= play.level && c.attackBonus !== null)
    .reduce<number | null>((best, c) => Math.max(best ?? -99, c.attackBonus!), null)
  const worn = play.inventory.filter((i) => i.equipped)
  const bag = play.inventory.filter((i) => !i.equipped)

  async function carry(item: BagItem, equipped: boolean) {
    setBusy(item.key)
    setFailed(false)
    try {
      onChanged(await equipItem(campaignId, item.key, equipped))
    } catch {
      setFailed(true)
    } finally {
      setBusy(null)
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <LevelUpPanel campaignId={campaignId} play={play} onChanged={onChanged} />
      <div className="flex items-end gap-3.5 rounded-[14px] border border-line bg-surface p-3.5">
        {character.sheet.look && <Sprite look={character.sheet.look} scale={4} label={name} />}
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <span className="type-title text-[22px]">{name}</span>
          <span className="text-caption text-mute-soft">
            {[character.peopleName, character.className, t('play.sheet.level', { level: play.level })]
              .filter(Boolean)
              .join(' · ')}
          </span>
          <CellBar stat="init" value={play.xpBar} max={play.xpBarMax} height={8} animated={false} label={t('play.sheet.xpBar')} />
          <span className="text-caption text-mute">
            {t('play.sheet.xp', { count: play.xpBarMax - play.xpBar })}
            {play.nextLevelXp !== null && ` · ${t('play.sheet.nextLevel', { level: play.level + 1, xp: play.nextLevelXp, total: play.totalXp })}`}
          </span>
          {play.upgradePoints > 0 && (
            <span className="text-caption font-bold">{t('play.sheet.upgrade', { count: play.upgradePoints })}</span>
          )}
        </div>
      </div>

      <section className="flex flex-col gap-2">
        <h2 className="type-label">{t('play.sheet.life')}</h2>
        <div className="flex flex-wrap items-center gap-3">
          <Hearts hp={play.hitPoints} max={play.maxHitPoints} px={2} />
          <span className="text-body font-bold">{t('play.sheet.hp', { hp: play.hitPoints, max: play.maxHitPoints })}</span>
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="type-label">{t('play.sheet.what')}</h2>
        <div className="flex flex-wrap items-end gap-3.5">
          <Gem label={t('play.sheet.ac')} stat="ac" value={String(play.armorClass)} />
          {attack !== null && <Gem label={t('play.sheet.atk')} stat="atk" value={signed(attack)} />}
          <Gem label={t('play.sheet.init')} stat="init" value={signed(play.initiative)} />
        </div>
        <ul className="grid grid-cols-3 gap-1.5">
          {play.abilities.map((a) => (
            <li key={a.id} className="flex flex-col items-center rounded-button border border-line bg-well px-2 py-1.5">
              <span className="text-[10px] font-semibold tracking-[0.12em] text-mute uppercase">{a.name}</span>
              <b className="text-body">{a.score}</b>
              <span className="text-caption text-mute-soft">{signed(a.modifier)}</span>
            </li>
          ))}
        </ul>
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="type-label">{t('play.sheet.cards')}</h2>
        <div className="-mx-4 flex gap-2 overflow-x-auto px-4 pb-2">
          {play.cards.map((c) => (
            <div key={c.id} className="flex flex-none flex-col items-center gap-1">
              <ActionCard card={c} width={96} level={play.level} />
              {c.level > play.level && (
                <span className="text-caption text-mute">{t('play.sheet.locked', { level: c.level })}</span>
              )}
            </div>
          ))}
        </div>
      </section>

      {play.resources.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="type-label">{t('play.sheet.purse')}</h2>
          <p className="flex flex-wrap gap-3 text-body">
            {play.resources.map((r) => (
              <span key={r.id}>
                <b>{r.amount}</b> {r.name}
              </span>
            ))}
          </p>
        </section>
      )}

      <section className="flex flex-col gap-2">
        <h2 className="type-label">{t('play.sheet.worn')}</h2>
        {worn.length === 0 ? (
          <p className="text-caption text-mute">{t('play.sheet.nothingWorn')}</p>
        ) : (
          <ItemList items={worn} action={t('play.sheet.unequip')} busy={busy} onAct={(i) => void carry(i, false)} />
        )}
        <p className="text-caption text-mute">{t('play.sheet.wornNote')}</p>
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="type-label">{t('play.sheet.bag')}</h2>
        {bag.length === 0 ? (
          <p className="text-caption text-mute">{t('play.sheet.emptyBag')}</p>
        ) : (
          <ItemList items={bag} action={t('play.sheet.equip')} busy={busy} onAct={(i) => void carry(i, true)} />
        )}
      </section>
      {failed && <p role="alert">{t('play.error')}</p>}
    </div>
  )
}

function Gem({ label, stat, value }: { label: string; stat: 'ac' | 'atk' | 'init'; value: string }) {
  return (
    <span className="flex flex-col items-center gap-1 text-[10px] font-semibold tracking-[0.12em] text-mute uppercase">
      <StatGem stat={stat} value={value} size={40} animated={false} />
      {label}
    </span>
  )
}

function ItemList({
  items,
  action,
  busy,
  onAct,
}: {
  items: BagItem[]
  action: string
  busy: string | null
  onAct: (item: BagItem) => void
}) {
  const { t } = useTranslation()
  return (
    <ul className="flex flex-col gap-1.5">
      {items.map((item) => (
        <li key={item.key} className="flex items-center gap-3 rounded-button border border-line bg-well px-3 py-2.5">
          <span className="flex min-w-0 flex-1 flex-col">
            <b className="text-body">
              {item.name}
              {item.qty > 1 && <span className="text-mute-soft"> ×{item.qty}</span>}
            </b>
            {item.description && <small className="text-caption text-mute-soft">{item.description}</small>}
            {item.consumable && <small className="text-caption text-mute">{t('play.sheet.consumable')}</small>}
          </span>
          <Button
            variant="outline"
            size="sm"
            aria-label={`${action} · ${item.name}`}
            disabled={busy !== null}
            onClick={() => onAct(item)}
          >
            {action}
          </Button>
        </li>
      ))}
    </ul>
  )
}
