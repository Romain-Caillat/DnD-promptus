import { useEffect, useRef, useState } from 'react'
import { cn } from '@/lib/utils'
import { usePrefersReducedMotion } from '@/lib/usePrefersReducedMotion'

const TICK_MS = 60

/**
 * A die that tumbles while its number rolls, then settles on `value`.
 * The tumble is the CSS `animate-tumble` token, and the number settles on
 * its `animationend`, so the timing lives in one place. Under reduced
 * motion there is no animation and so no `animationend`: the die shows
 * its result at once instead of freezing on a random face.
 * Remount it (change its `key`) to roll again.
 */
export function RollingDie({
  value,
  faces,
  className,
}: {
  value: number
  faces: number
  className?: string
}) {
  const reduced = usePrefersReducedMotion()
  const [settled, setSettled] = useState(false)
  const [face, setFace] = useState(() => randomFace(faces))
  const rolling = !reduced && !settled
  const dieRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!rolling) return
    const id = window.setInterval(() => setFace(randomFace(faces)), TICK_MS)
    // A native listener: React's onAnimationEnd guesses a vendor-prefixed
    // event name where `animation` is missing from the style object.
    const die = dieRef.current
    const settle = () => setSettled(true)
    die?.addEventListener('animationend', settle)
    return () => {
      window.clearInterval(id)
      die?.removeEventListener('animationend', settle)
    }
  }, [rolling, faces])

  return (
    <div
      className={cn('relative grid size-21 place-items-center', rolling && 'animate-tumble', className)}
      ref={dieRef}
      data-rolling={rolling}
    >
      <span
        aria-hidden
        className="absolute inset-0 bg-current [clip-path:polygon(50%_0,94%_25%,94%_75%,50%_100%,6%_75%,6%_25%)]"
      />
      <span
        aria-hidden
        className="absolute inset-[22%_18%_26%] bg-white/45 [clip-path:polygon(50%_0,100%_100%,0_100%)]"
      />
      <span className="relative mt-3 text-2xl font-bold text-ink tabular-nums">
        {rolling ? face : value}
      </span>
    </div>
  )
}

function randomFace(faces: number) {
  return 1 + Math.floor(Math.random() * faces)
}
