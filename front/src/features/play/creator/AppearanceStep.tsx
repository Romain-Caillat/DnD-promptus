import { useTranslation } from 'react-i18next'
import { Sprite } from '@/features/sprites/Sprite'
import type { CharacterLook } from '@/features/sprites/look'
import { randomLook, type CataloguePiece, type PackCatalogue, type Swatch } from '@/lib/creator'

const APPEARANCE_TABS = ['body', 'outfit', 'colours'] as const
export type AppearanceTab = (typeof APPEARANCE_TABS)[number]

type Worn = NonNullable<CharacterLook['outfit']>
type WornSlot = 'headwear' | 'outfit' | 'armour' | 'weapon'

/** The worn slots whose pieces take colours, in the order the colours tab lists them. */
const DYED_SLOTS = ['outfit', 'headwear', 'armour'] as const satisfies readonly WornSlot[]

/** A worn piece in its full form. */
function full(w: Worn | undefined): { piece: string; dye?: string; accent?: string } | undefined {
  if (w === undefined) return undefined
  return typeof w === 'string' ? { piece: w } : w
}

/**
 * The layered configurator (board « Créer », moments 2 to 4): the
 * character drawn live by the server, then the pieces of the pack, slot
 * by slot, and their colours. Three tabs: body, outfit, colours.
 */
export function AppearanceStep({
  pack,
  look,
  onLook,
  tab,
  onTab,
  name,
  onName,
}: {
  pack: PackCatalogue
  look: CharacterLook
  onLook: (look: CharacterLook) => void
  tab: AppearanceTab
  onTab: (tab: AppearanceTab) => void
  name: string
  onName: (name: string) => void
}) {
  const { t } = useTranslation()
  const set = (patch: Partial<CharacterLook>) => onLook({ ...look, ...patch })
  const piece = (slot: WornSlot, id: string | undefined) => {
    if (id === undefined) return set({ [slot]: undefined })
    const old = full(look[slot])
    set({ [slot]: { piece: id, dye: old?.dye, accent: old?.accent } })
  }
  const colour = (slot: WornSlot, key: 'dye' | 'accent', id: string) => {
    const old = full(look[slot])
    if (old) set({ [slot]: { ...old, [key]: id } })
  }
  const accessories = (look.accessories ?? []).map((a) => full(a)!)
  const toggleAccessory = (id: string) =>
    set({
      accessories: accessories.some((a) => a.piece === id)
        ? accessories.filter((a) => a.piece !== id)
        : [...accessories, { piece: id }],
    })
  const worn = (slot: WornSlot) => full(look[slot])
  const catalogued = (slot: WornSlot) => {
    const w = worn(slot)
    return w && pack.slots[slot]?.find((p) => p.id === w.piece)
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="cr-stage">
        <span className="cr-stage-label type-label">{name || t('creator.look.yours')}</span>
        <Sprite look={look} scale={8} label={name || t('creator.look.yours')} className="cr-stage-sprite" />
      </div>
      <div className="cr-tabs" role="tablist">
        {APPEARANCE_TABS.map((k) => (
          <button key={k} type="button" role="tab" aria-selected={tab === k} onClick={() => onTab(k)}>
            {t(`creator.look.tabs.${k}`)}
          </button>
        ))}
      </div>

      {tab === 'body' && (
        <>
          <Pieces
            label={t('creator.look.body')}
            pieces={pack.slots.body}
            selected={look.body}
            onPick={(id) => id && set({ body: id })}
          />
          <Swatches
            label={t('creator.look.skin')}
            swatches={pack.palettes.skin}
            selected={look.skin}
            onPick={(id) => set({ skin: id })}
          />
          <Pieces
            label={t('creator.look.hair')}
            pieces={pack.slots.hair}
            selected={look.hair.style}
            onPick={(id) => set({ hair: { ...look.hair, style: id } })}
            none={t('creator.look.noHair')}
          />
          <Pieces
            label={t('creator.look.beard')}
            pieces={pack.slots.beard}
            selected={look.beard}
            onPick={(id) => set({ beard: id })}
            none={t('creator.look.none')}
          />
        </>
      )}

      {tab === 'outfit' && (
        <>
          <Pieces
            label={t('creator.look.headwear')}
            pieces={pack.slots.headwear}
            selected={worn('headwear')?.piece}
            onPick={(id) => piece('headwear', id)}
            none={t('creator.look.none')}
          />
          <Pieces
            label={t('creator.look.outfit')}
            pieces={pack.slots.outfit}
            selected={worn('outfit')?.piece}
            onPick={(id) => piece('outfit', id)}
            none={t('creator.look.none')}
          />
          <Pieces
            label={t('creator.look.armour')}
            pieces={pack.slots.armour}
            selected={worn('armour')?.piece}
            onPick={(id) => piece('armour', id)}
            none={t('creator.look.none')}
          />
          <Pieces
            label={t('creator.look.weapon')}
            pieces={pack.slots.weapon}
            selected={worn('weapon')?.piece}
            onPick={(id) => piece('weapon', id)}
            none={t('creator.look.none')}
          />
          {pack.slots.accessory?.length > 0 && (
            <div className="flex flex-col gap-2">
              <span className="type-label">{t('creator.look.accessories')}</span>
              <div className="cr-chips">
                {pack.slots.accessory.map((p) => (
                  <button
                    key={p.id}
                    type="button"
                    className="cr-chip"
                    aria-pressed={accessories.some((a) => a.piece === p.id)}
                    onClick={() => toggleAccessory(p.id)}
                  >
                    {p.name}
                  </button>
                ))}
              </div>
            </div>
          )}
        </>
      )}

      {tab === 'colours' && (
        <>
          <Swatches
            label={t('creator.look.hairColour')}
            swatches={pack.palettes.hair}
            selected={look.hair.colour}
            onPick={(id) => set({ hair: { ...look.hair, colour: id } })}
          />
          {DYED_SLOTS.map((slot) => {
            const p = catalogued(slot)
            const w = worn(slot)
            if (!p || !w) return null
            return (
              <div key={slot} className="flex flex-col gap-3">
                {p.dyed && (
                  <Swatches
                    label={t(`creator.look.dye.${slot}`, { piece: p.name })}
                    swatches={pack.palettes.cloth}
                    selected={w.dye}
                    onPick={(id) => colour(slot, 'dye', id)}
                  />
                )}
                {p.accented && (
                  <Swatches
                    label={t('creator.look.accent', { piece: p.name })}
                    swatches={pack.palettes.cloth}
                    selected={w.accent}
                    onPick={(id) => colour(slot, 'accent', id)}
                  />
                )}
              </div>
            )
          })}
          <label className="flex flex-col gap-1.5">
            <span className="type-label">{t('creator.look.name')}</span>
            <input
              className="cr-field"
              value={name}
              maxLength={60}
              autoComplete="off"
              placeholder={t('creator.look.namePlaceholder')}
              onChange={(e) => onName(e.target.value)}
            />
          </label>
        </>
      )}

      <button type="button" className="cr-dice" onClick={() => onLook(randomLook(pack))}>
        {t('creator.look.random')}
      </button>
    </div>
  )
}

function Pieces({
  label,
  pieces = [],
  selected,
  onPick,
  none,
}: {
  label: string
  pieces?: CataloguePiece[]
  selected: string | undefined
  onPick: (id: string | undefined) => void
  /** The label of « no piece », when the slot may stay empty. */
  none?: string
}) {
  if (pieces.length === 0) return null
  return (
    <div className="flex flex-col gap-2">
      <span className="type-label">{label}</span>
      <div className="cr-chips">
        {none && (
          <button type="button" className="cr-chip" aria-pressed={selected === undefined} onClick={() => onPick(undefined)}>
            {none}
          </button>
        )}
        {pieces.map((p) => (
          <button
            key={p.id}
            type="button"
            className="cr-chip"
            aria-pressed={selected === p.id}
            onClick={() => onPick(p.id)}
          >
            {p.name}
          </button>
        ))}
      </div>
    </div>
  )
}

function Swatches({
  label,
  swatches,
  selected,
  onPick,
}: {
  label: string
  swatches: Swatch[]
  selected: string | undefined
  onPick: (id: string) => void
}) {
  return (
    <div className="flex flex-col gap-2">
      <span className="type-label">{label}</span>
      <div className="cr-chips">
        {swatches.map((s) => (
          <button
            key={s.id}
            type="button"
            className="cr-swatch"
            style={{ background: s.colour }}
            aria-label={s.name}
            aria-pressed={selected === s.id}
            onClick={() => onPick(s.id)}
          />
        ))}
      </div>
    </div>
  )
}
