import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { CardButton } from '@/components/game/CardButton'
import { cn } from '@/lib/utils'
import { usePrefersReducedMotion } from '@/lib/usePrefersReducedMotion'
import { SpritesSection } from '@/features/sprites/SpritesSection'
import { GameComponentsSection } from './GameComponentsSection'
import { RollingDie } from './RollingDie'
import { RARITIES, RARITY_MATERIAL, type Rarity } from '@/components/game/rarity'
import { STAT_BG, STAT_TEXT, STATS, shapeMask, type Stat } from '@/components/game/stats'

/**
 * The « Fondations » board of the design canvas, rebuilt from the real
 * tokens (styles/tokens.css): palette, stat colours and shapes, type
 * scale, card and rarity materials, motion. Values shown next to each
 * token are read back from the stylesheet, never copied here.
 */
export function ReferencePage() {
  const { t } = useTranslation()

  return (
    <main className="surface-table min-h-dvh text-chalk">
      <div className="mx-auto flex max-w-6xl flex-col gap-5 px-4 py-10 md:px-16 md:py-14">
        <header className="flex flex-col gap-4">
          <p className="type-label">{t('reference.kicker')}</p>
          <h1 className="type-title text-[3.5rem] leading-[0.95] md:text-display">{t('app.name')}</h1>
          <p className="type-narration max-w-2xl text-narration text-chalk-soft md:text-narration-lg">
            {t('reference.intro')}
          </p>
        </header>

        <MenuSection />
        <div className="grid gap-5 lg:grid-cols-2">
          <PaletteSection />
          <TypeSection />
        </div>
        <StatsSection />
        <SpritesSection />
        <MaterialsSection />
        <MotionSection />
        <GameComponentsSection />
      </div>
    </main>
  )
}

/** Reads a token back from the stylesheet (empty when no CSS is loaded). */
function readToken(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim()
}

function Slab({
  title,
  note,
  className,
  children,
}: {
  title: string
  note?: string
  className?: string
  children: ReactNode
}) {
  return (
    <section className={cn('surface-slab flex flex-col gap-4 p-5', className)}>
      <h2 className="type-label">{title}</h2>
      {children}
      {note && <p className="text-caption text-mute">{note}</p>}
    </section>
  )
}

// --- Pixel menu ----------------------------------------------------------

const CHOICES = ['attack', 'item', 'endTurn'] as const
const TABS = ['text', 'combat', 'visuals', 'sound'] as const
/** The pixel font's crisp sizes (one font pixel = 1/8 of the size). */
const PIXEL_SIZES = [12, 16, 24, 32] as const
const PX = ' px'

/**
 * The interface chrome (board « Pistes UI », track A, ui/adopt-pixel-menu):
 * keys, the RPG cursor on a choice, tabs, a field, the pixel font's sizes.
 */
function MenuSection() {
  const { t } = useTranslation()
  const [choice, setChoice] = useState<(typeof CHOICES)[number]>('attack')
  const [tab, setTab] = useState<(typeof TABS)[number]>('text')
  return (
    <Slab title={t('reference.menu.title')} note={t('reference.menu.note')}>
      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
        <div className="flex flex-col gap-3">
          <h3 className="type-label">{t('reference.menu.keys')}</h3>
          <div className="flex flex-wrap items-center gap-2.5">
            <Button size="lg">{t('reference.menu.main')}</Button>
            <Button variant="outline" size="lg">
              {t('reference.menu.other')}
            </Button>
          </div>
          <div className="flex flex-wrap items-center gap-2.5">
            <Button>{t('reference.menu.main')}</Button>
            <Button variant="outline">{t('reference.menu.other')}</Button>
            <Button variant="ghost">{t('reference.menu.quiet')}</Button>
            <Button variant="destructive">{t('reference.menu.danger')}</Button>
          </div>
          <p className="text-caption text-mute">{t('reference.menu.keysNote')}</p>
        </div>

        <div className="flex flex-col gap-3">
          <h3 className="type-label">{t('reference.menu.choice')}</h3>
          <div className="flex flex-col gap-2.5">
            {CHOICES.map((c) => (
              <Button
                key={c}
                size="lg"
                variant={c === choice ? 'default' : 'outline'}
                aria-pressed={c === choice}
                className="w-full"
                onClick={() => setChoice(c)}
              >
                {t(`reference.menu.${c}`)}
              </Button>
            ))}
          </div>
          <p className="text-caption text-mute">{t('reference.menu.choiceNote')}</p>
        </div>

        <div className="flex flex-col gap-5">
          <div className="flex flex-col gap-3">
            <h3 className="type-label">{t('reference.menu.tabs')}</h3>
            <div role="tablist" aria-label={t('reference.menu.tabs')} className="grid grid-cols-2 gap-1 sm:grid-cols-4 md:grid-cols-2">
              {TABS.map((id) => (
                <button
                  key={id}
                  type="button"
                  role="tab"
                  aria-selected={tab === id}
                  className="pixel-tab min-h-9 pb-1"
                  onClick={() => setTab(id)}
                >
                  {t(`reference.menu.tabNames.${id}`)}
                </button>
              ))}
            </div>
          </div>
          <label className="flex flex-col gap-1.5">
            <span className="type-label">{t('reference.menu.fieldLabel')}</span>
            <input className="pixel-field px-2.5 py-2 text-body" defaultValue={t('reference.menu.fieldValue')} />
            <span className="text-caption text-mute">{t('reference.menu.fieldNote')}</span>
          </label>
        </div>
      </div>

      <div className="flex flex-col gap-2 border-t border-line pt-4">
        <h3 className="type-label">{t('reference.menu.sizes')}</h3>
        <ul className="flex flex-col gap-1.5">
          {PIXEL_SIZES.map((size) => (
            <li key={size} className="flex items-baseline gap-4">
              <span className="w-12 flex-none text-caption text-mute tabular-nums">
                {size}
                {PX}
              </span>
              <span className="type-title min-w-0 truncate" style={{ fontSize: size }}>
                {t('reference.menu.sample')}
              </span>
            </li>
          ))}
        </ul>
        <p className="text-caption text-mute">{t('reference.menu.sizesNote')}</p>
      </div>
    </Slab>
  )
}

// --- Palette -------------------------------------------------------------

const PALETTE = [
  { token: 'table-deep', key: 'reference.palette.tableDeep', bg: 'bg-table-deep' },
  { token: 'table', key: 'reference.palette.table', bg: 'bg-table' },
  { token: 'well', key: 'reference.palette.well', bg: 'bg-well' },
  { token: 'surface', key: 'reference.palette.surface', bg: 'bg-surface' },
  { token: 'slab', key: 'reference.palette.slab', bg: 'bg-slab' },
  { token: 'surface-raised', key: 'reference.palette.surfaceRaised', bg: 'bg-surface-raised' },
  { token: 'line', key: 'reference.palette.line', bg: 'bg-line' },
  { token: 'line-strong', key: 'reference.palette.lineStrong', bg: 'bg-line-strong' },
  { token: 'line-dashed', key: 'reference.palette.lineDashed', bg: 'bg-line-dashed' },
  { token: 'mute', key: 'reference.palette.mute', bg: 'bg-mute' },
  { token: 'mute-soft', key: 'reference.palette.muteSoft', bg: 'bg-mute-soft' },
  { token: 'chalk-soft', key: 'reference.palette.chalkSoft', bg: 'bg-chalk-soft' },
  { token: 'chalk', key: 'reference.palette.chalk', bg: 'bg-chalk' },
  { token: 'ivory', key: 'reference.palette.ivory', bg: 'bg-ivory' },
  { token: 'ivory-edge', key: 'reference.palette.ivoryEdge', bg: 'bg-ivory-edge' },
  { token: 'ink', key: 'reference.palette.ink', bg: 'bg-ink' },
] as const

function PaletteSection() {
  const { t } = useTranslation()
  return (
    <Slab title={t('reference.palette.title')} note={t('reference.palette.note')}>
      <ul className="grid grid-cols-4 gap-2 sm:grid-cols-8 lg:grid-cols-4">
        {PALETTE.map((c) => (
          <li key={c.token} className="flex flex-col gap-1.5">
            <span className={cn('h-10 rounded-frame border border-line', c.bg)} />
            <span className="text-caption leading-tight text-chalk-soft">{t(c.key)}</span>
            <code className="text-label text-mute">{readToken(`--color-${c.token}`)}</code>
          </li>
        ))}
      </ul>
      <div className="flex gap-1.5">
        {STATS.map((s) => (
          <span key={s} className={cn('h-7 flex-1 rounded-md', STAT_BG[s])} />
        ))}
      </div>
    </Slab>
  )
}

// --- Stats ---------------------------------------------------------------

function StatShape({ stat, cells = 9, size = 54 }: { stat: Stat; cells?: number; size?: number }) {
  return (
    <span
      aria-hidden
      className="grid drop-shadow-gem"
      style={{ width: size, height: size, gridTemplateColumns: `repeat(${cells}, minmax(0, 1fr))` }}
    >
      {shapeMask(stat, cells).map((on, i) => (
        <span key={i} className={cn('m-px', on && STAT_BG[stat])} />
      ))}
    </span>
  )
}

const DAMAGE = '−6'
/** Separator between two values, not a word. */
const SEP = ' · '

function StatsSection() {
  const { t } = useTranslation()
  return (
    <Slab title={t('reference.stats.title')} note={t('reference.stats.note')}>
      <ul className="grid grid-cols-2 gap-x-3 gap-y-5 sm:grid-cols-3 lg:grid-cols-6">
        {STATS.map((s) => (
          <li key={s} className="flex flex-col items-center gap-2 text-center">
            <StatShape stat={s} />
            <span className={cn('type-label', STAT_TEXT[s])}>{t(`reference.stats.${s}.code`)}</span>
            <span className="text-body leading-tight">{t(`reference.stats.${s}.name`)}</span>
            <span className="text-caption text-mute">
              {t(`reference.stats.${s}.shape`)}{SEP}{readToken(`--color-stat-${s}`)}
            </span>
          </li>
        ))}
      </ul>
      <div className="flex items-center gap-5 border-t border-line pt-4">
        <span className="damage-number text-[2rem]">{DAMAGE}</span>
        <div className="flex flex-col gap-1">
          <span className="type-label text-damage">{t('reference.stats.damage')}</span>
          <span className="text-caption text-mute-soft">{t('reference.stats.damageNote')}</span>
        </div>
      </div>
    </Slab>
  )
}

// --- Type ----------------------------------------------------------------

const SCALE = [
  { key: 'display', className: 'type-title text-[3rem] leading-none md:text-display' },
  { key: 'headingLg', className: 'type-title text-heading-lg' },
  { key: 'heading', className: 'type-title text-heading' },
  { key: 'cardTitle', className: 'type-title text-card-title' },
  { key: 'narrationLg', className: 'type-narration text-narration-lg' },
  { key: 'narration', className: 'type-narration text-narration' },
  { key: 'stat', className: 'text-stat' },
  { key: 'body', className: 'text-body' },
  { key: 'caption', className: 'text-caption' },
  { key: 'label', className: 'type-label text-chalk' },
] as const

const SAMPLE_STATS = { hp: '18/22', ac: '16', atk: '+5' }

function TypeSection() {
  const { t } = useTranslation()
  return (
    <Slab title={t('reference.type.title')} note={t('reference.type.note')}>
      <div className="flex flex-col gap-2">
        <p className="type-title text-heading-lg">{t('reference.type.sampleTitle')}</p>
        <p className="type-narration text-narration text-chalk-soft">{t('reference.type.sampleNarration')}</p>
        <p className="text-stat">
          <span className="text-stat-hp">{t('reference.stats.hp.code')} {SAMPLE_STATS.hp}</span>
          {SEP}
          <span className="text-stat-ac">{t('reference.stats.ac.code')} {SAMPLE_STATS.ac}</span>
          {SEP}
          <span className="text-stat-atk">{t('reference.stats.atk.code')} {SAMPLE_STATS.atk}</span>
        </p>
      </div>
      <ul className="flex flex-col gap-3 border-t border-line pt-4">
        {SCALE.map((s) => (
          <li key={s.key} className="flex flex-col gap-0.5">
            <span className={cn('truncate', s.className)}>{t(`reference.type.scale.${s.key}.name`)}</span>
            <span className="text-caption text-mute">{t(`reference.type.scale.${s.key}.meta`)}</span>
          </li>
        ))}
      </ul>
    </Slab>
  )
}

// --- Materials -----------------------------------------------------------

const ATTACK_GEM = { stat: 'atk', value: '+5' } as const

function RarityCard({ tier, rank }: { tier: Rarity; rank: number }) {
  const { t } = useTranslation()
  return (
    <figure className="flex flex-col items-center gap-3">
      <div className={cn('relative aspect-[5/7] w-full max-w-30 rounded-[9%/6.4%]', RARITY_MATERIAL[tier])}>
        <div className="card-frame absolute inset-[5.5%] flex flex-col items-center justify-between rounded-[6%/4.2%] [background:var(--frame-bg)] px-1 py-3 text-center">
          <span className="type-card text-[13px] leading-tight">{t(`reference.materials.tiers.${tier}.name`)}</span>
          <span
            className="flex gap-1.5"
            role="img"
            aria-label={t('reference.materials.pips', { count: rank })}
          >
            {Array.from({ length: rank }, (_, i) => (
              <i key={i} className="size-1.5 rotate-45 bg-(--pip-color)" />
            ))}
          </span>
        </div>
      </div>
      <figcaption className="text-center text-caption text-mute">
        {t(`reference.materials.tiers.${tier}.how`)}
      </figcaption>
    </figure>
  )
}

function MaterialsSection() {
  const { t } = useTranslation()
  return (
    <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_minmax(0,2fr)]">
      <Slab title={t('reference.materials.buttons')} note={t('reference.materials.buttonsNote')}>
        <div className="flex max-w-[300px] flex-col gap-4">
          <CardButton
            title={t('reference.materials.attack')}
            subtitle={t('reference.materials.attackSub')}
            gem={ATTACK_GEM}
          />
          <CardButton
            title={t('reference.materials.endTurn')}
            subtitle={t('reference.materials.endTurnSub')}
            variant="dark"
            icon="hourglass"
          />
          <div className="grid grid-cols-2 gap-2.5">
            <CardButton title={t('reference.materials.accept')} size="small" />
            <CardButton title={t('reference.materials.refuse')} size="small" variant="dark" />
          </div>
        </div>
      </Slab>
      <Slab title={t('reference.materials.rarity')} note={t('reference.materials.rarityNote')}>
        <div className="grid grid-cols-3 gap-4 sm:grid-cols-6">
          {RARITIES.map((tier, i) => (
            <RarityCard key={tier} tier={tier} rank={i + 1} />
          ))}
        </div>
      </Slab>
    </div>
  )
}

// --- Motion --------------------------------------------------------------

const DURATIONS = [
  'press',
  'flip',
  'deal',
  'pop',
  'roll',
  'heartbeat',
  'bob',
  'gemWave',
  'sheen',
] as const

const EASINGS = ['deal', 'settle', 'roll'] as const

/** `gemWave` → `--duration-gem-wave` */
function cssName(key: string) {
  return key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)
}

const ROLL = { value: 17, faces: 20 }

function MotionSection() {
  const { t } = useTranslation()
  const reduced = usePrefersReducedMotion()
  // Remounts the one-shot samples (deal, pop, roll) to play them again.
  const [take, setTake] = useState(0)

  return (
    <Slab title={t('reference.motion.title')} note={t('reference.motion.note')}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-caption text-mute-soft" role="status">
          {reduced ? t('reference.motion.reducedOn') : t('reference.motion.reducedOff')}
        </p>
        <Button variant="outline" onClick={() => setTake((n) => n + 1)}>
          {t('reference.motion.replay')}
        </Button>
      </div>

      <ul className="grid grid-cols-2 items-end gap-6 sm:grid-cols-3 lg:grid-cols-6">
        <MotionSample label={t('reference.motion.samples.deal')}>
          <div key={take} className="animate-deal">
            <div className="material-common relative aspect-[5/7] w-16 rounded-[9%/6.4%]">
              <div className="card-frame absolute inset-[5.5%] rounded-[6%/4.2%]" />
            </div>
          </div>
        </MotionSample>

        <MotionSample label={t('reference.motion.samples.press')}>
          <div className="button-card w-24 animate-press-demo">
            <div className="card-frame h-8" />
          </div>
        </MotionSample>

        <MotionSample label={t('reference.motion.samples.pop')}>
          <span key={take} className="damage-number inline-block animate-pop text-[2rem]">
            {DAMAGE}
          </span>
        </MotionSample>

        <MotionSample label={t('reference.motion.samples.roll')}>
          <RollingDie key={take} value={ROLL.value} faces={ROLL.faces} className="text-stat-atk" />
        </MotionSample>

        <MotionSample label={t('reference.motion.samples.bob')}>
          <span className="grid size-12 animate-bob place-items-center rounded-full bg-ivory shadow-ivory-flat" />
        </MotionSample>

        <MotionSample label={t('reference.motion.samples.sheen')}>
          <div className="material-epic aspect-[5/7] w-16 rounded-[9%/6.4%]" />
        </MotionSample>
      </ul>

      <div className="grid gap-5 border-t border-line pt-4 md:grid-cols-2">
        <TokenTable
          title={t('reference.motion.durationsTitle')}
          rows={DURATIONS.map((d) => ({
            key: d,
            label: t(`reference.motion.durations.${d}`),
            value: readToken(`--duration-${cssName(d)}`),
          }))}
        />
        <TokenTable
          title={t('reference.motion.easingsTitle')}
          rows={EASINGS.map((e) => ({
            key: e,
            label: t(`reference.motion.easings.${e}`),
            value: readToken(`--ease-${e}`),
          }))}
        />
      </div>
    </Slab>
  )
}

function MotionSample({ label, children }: { label: string; children: ReactNode }) {
  return (
    <li className="flex flex-col items-center gap-3">
      <div className="grid h-24 place-items-center">{children}</div>
      <span className="text-center text-caption text-mute-soft">{label}</span>
    </li>
  )
}

function TokenTable({
  title,
  rows,
}: {
  title: string
  rows: { key: string; label: string; value: string }[]
}) {
  return (
    <div className="flex flex-col gap-2">
      <h3 className="type-label">{title}</h3>
      <dl className="flex flex-col gap-1">
        {rows.map((r) => (
          <div key={r.key} className="flex justify-between gap-4 rounded-frame bg-well px-3 py-1.5 text-body">
            <dt className="text-chalk-soft">{r.label}</dt>
            <dd className="text-mute tabular-nums">{r.value}</dd>
          </div>
        ))}
      </dl>
    </div>
  )
}
