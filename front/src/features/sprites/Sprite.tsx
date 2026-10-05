import type { CSSProperties } from 'react'
import { cn } from '@/lib/utils'
import {
  SPRITE_HEIGHT,
  SPRITE_WIDTH,
  spriteUrl,
  type CharacterLook,
  type ConditionVisual,
  type Facing,
} from './look'
import './sprite.css'

/** A particle of an effect: position in % of the sprite, size in sprite pixels. */
interface Particle {
  x: number
  y: number
  w: number
  h: number
  delay?: number
  colour?: string
  rotate?: number
  text?: string
}

// Ported from the « Personnage et état » component of the design canvas.
const PARTICLES: Partial<Record<ConditionVisual, Particle[]>> = {
  poison: [
    [30, 40, 2, 0],
    [60, 30, 3, 0.5],
    [45, 55, 2, 0.9],
    [70, 50, 2, 1.3],
  ].map(([x, y, s, delay]) => ({ x, y, w: s, h: s, delay })),
  etourdi: [
    [0, 30],
    [80, 30],
    [42, 80],
  ].map(([x, y]) => ({ x, y, w: 3, h: 3 })),
  endormi: [
    [62, 4, 0, 3],
    [70, -4, 0.8, 4],
    [56, 0, 1.6, 5],
  ].map(([x, y, delay, s]) => ({ x, y, w: s, h: s, delay, text: 'z' })),
  feu: (
    [
      [22, 62, '#FF6A00'],
      [42, 50, '#FFD27A'],
      [60, 58, '#E0401A'],
      [34, 70, '#FFD27A'],
      [54, 72, '#FF6A00'],
    ] as const
  ).map(([x, y, colour], i) => ({ x, y, w: 3, h: 6, colour, delay: i * 0.12 })),
  entrave: [
    { x: -6, y: 48, w: SPRITE_WIDTH * 1.12, h: 2, rotate: 18 },
    { x: -6, y: 62, w: SPRITE_WIDTH * 1.12, h: 2, rotate: -14 },
  ],
  effraye: [{ x: 74, y: 6, w: 2, h: 3 }],
  beni: [
    [2, 20, 0],
    [84, 34, 0.6],
    [10, 64, 1.2],
    [80, 74, 0.9],
  ].map(([x, y, delay]) => ({ x, y, w: 3, h: 3, delay })),
  cible: [{ x: 38, y: -14, w: 5, h: 5 }],
}

/** Effects whose particles orbit together (the stars of a stunned character). */
const RING: ConditionVisual[] = ['etourdi']

/**
 * A character drawn from its description. The server renders the PNG
 * (one renderer everywhere); this enlarges it by a whole number, pixel
 * crisp, and draws each condition's effect over it.
 */
export function Sprite({
  look,
  scale = 4,
  facing = 'east',
  effects = [],
  label,
  className,
}: {
  look: CharacterLook
  /** Screen pixels per sprite pixel. */
  scale?: number
  facing?: Facing
  /** Condition effects shown on the character, from the rule system. */
  effects?: readonly ConditionVisual[]
  /** Who it is, for screen readers; without it the sprite is decorative. */
  label?: string
  className?: string
}) {
  const width = SPRITE_WIDTH * scale
  const height = SPRITE_HEIGHT * scale
  return (
    <span
      className={cn('sprite', effects.map((e) => `sprite-fx-${e}`), className)}
      style={{ width, height }}
      data-effects={effects.join(' ') || undefined}
    >
      <img
        className="sprite-body"
        src={spriteUrl(look, facing)}
        width={width}
        height={height}
        alt={label ?? ''}
        draggable={false}
      />
      {effects.includes('beni') && <span className="sprite-halo" aria-hidden />}
      {effects.map((effect) => (
        <Particles key={effect} effect={effect} scale={scale} />
      ))}
    </span>
  )
}

function Particles({ effect, scale }: { effect: ConditionVisual; scale: number }) {
  const parts = PARTICLES[effect]
  if (!parts) return null
  const spans = parts.map((p, i) => {
    const style: CSSProperties = {
      left: `${p.x}%`,
      top: `${p.y}%`,
      width: p.text ? undefined : p.w * scale,
      height: p.text ? undefined : p.h * scale,
      fontSize: p.text ? p.h * scale : undefined,
      background: p.colour,
      transform: p.rotate === undefined ? undefined : `rotate(${p.rotate}deg)`,
      animationDelay: p.delay ? `${p.delay}s` : undefined,
    }
    return (
      <span key={i} className={`sprite-particle sprite-particle-${effect}`} style={style}>
        {p.text}
      </span>
    )
  })
  if (RING.includes(effect)) {
    return (
      <span className="sprite-ring" aria-hidden>
        {spans}
      </span>
    )
  }
  return <span aria-hidden>{spans}</span>
}
