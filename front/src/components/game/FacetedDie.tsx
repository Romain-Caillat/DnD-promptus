import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { usePrefersReducedMotion } from '@/lib/usePrefersReducedMotion'
import { STAT_FILL_TEXT, type Stat } from './stats'

/** The dice the rules roll; d100 is two d10s read together. */
export const DICE = [4, 6, 8, 10, 12, 20, 100] as const
export type Faces = (typeof DICE)[number]

/** How long the die tumbles before it lands: `--duration-roll`. */
export const ROLL_MS = 1400
const TICK_MS = 60

/**
 * Each die's silhouette and its inner facets, in a 100×100 box: what
 * makes a d8 read as a d8 at a glance on a phone.
 */
const SHAPES: Record<Faces, { outline: string; facets: string[] }> = {
  4: { outline: '50,6 95,88 5,88', facets: ['50,6 50,62 5,88', '50,6 95,88 50,62'] },
  6: { outline: '12,12 88,12 88,88 12,88', facets: ['12,12 88,12 74,26 26,26', '88,12 88,88 74,74 74,26'] },
  8: { outline: '50,4 94,50 50,96 6,50', facets: ['50,4 94,50 6,50', '6,50 50,96 50,50'] },
  10: { outline: '50,4 94,42 50,96 6,42', facets: ['50,4 70,48 30,48', '6,42 30,48 50,96', '94,42 70,48 50,96'] },
  12: { outline: '50,4 93,35 77,90 23,90 7,35', facets: ['50,22 72,40 64,68 36,68 28,40'] },
  20: { outline: '50,3 92,27 92,73 50,97 8,73 8,27', facets: ['50,22 78,68 22,68', '50,3 50,22 8,27', '92,27 78,68 92,73'] },
  100: { outline: '50,4 94,42 50,96 6,42', facets: ['50,4 70,48 30,48', '6,42 30,48 50,96', '94,42 70,48 50,96'] },
}

/** `1d20` → 20; anything the shapes do not know reads as a d20. */
export function facesOf(die: string): Faces {
  const n = Number(die.split('d').pop())
  return (DICE as readonly number[]).includes(n) ? (n as Faces) : 20
}

function randomFace(faces: number) {
  return 1 + Math.floor(Math.random() * faces)
}

/**
 * A faceted die (ui/roll-faceted-dice): its silhouette for d4 to d20 and
 * d100, shaded facets tinted with the colour of the stat rolled (ivory
 * for a plain check). The numbers tumble, then land on `value` with a
 * flash. The value comes from the server: the die animates a roll
 * already made, it never decides one. Under reduced motion it shows the
 * value at once. Remount it (change its `key`) to roll again.
 */
export function FacetedDie({
  faces,
  value,
  stat,
  className,
}: {
  faces: Faces
  value: number
  stat?: Stat
  className?: string
}) {
  const { t } = useTranslation()
  const reduced = usePrefersReducedMotion()
  const [landed, setLanded] = useState(false)
  const [face, setFace] = useState(() => randomFace(faces))
  const rolling = !reduced && !landed

  useEffect(() => {
    if (!rolling) return
    const tick = window.setInterval(() => setFace(randomFace(faces)), TICK_MS)
    const land = window.setTimeout(() => setLanded(true), ROLL_MS)
    return () => {
      window.clearInterval(tick)
      window.clearTimeout(land)
    }
  }, [rolling, faces])

  const shape = SHAPES[faces]
  const shown = rolling ? face : value
  return (
    <div
      role="img"
      aria-label={t('dice.die', { faces, value })}
      data-rolling={rolling}
      className={cn(
        'relative grid size-21 place-items-center',
        stat ? STAT_FILL_TEXT[stat] : 'text-ivory',
        rolling && 'animate-tumble',
        !rolling && !reduced && 'animate-pop',
        className,
      )}
    >
      <svg viewBox="0 0 100 100" className="absolute inset-0 size-full" aria-hidden>
        <polygon points={shape.outline} fill="currentColor" stroke="var(--color-ink)" strokeWidth="3" />
        {shape.facets.map((p) => (
          <polygon key={p} points={p} fill="black" fillOpacity="0.16" />
        ))}
        <polygon points={shape.outline} fill="white" fillOpacity="0.14" clipPath="inset(0 50% 50% 0)" />
      </svg>
      <span aria-hidden className="relative text-2xl font-bold text-ink tabular-nums" data-testid="die-face">
        {faces === 100 ? String(shown).padStart(2, '0') : shown}
      </span>
    </div>
  )
}
