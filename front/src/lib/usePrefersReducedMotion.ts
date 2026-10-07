import { useMediaQuery } from './useMediaQuery'

/**
 * Whether the device asks for reduced motion, kept live. CSS animations
 * are switched off globally in `styles/tokens.css`; motion driven from
 * JavaScript (a number that rolls, a timer that waits for an
 * `animationend`) must read this and jump straight to its calm, settled
 * state instead (MEMORY.md §4).
 */
export function usePrefersReducedMotion(): boolean {
  return useMediaQuery('(prefers-reduced-motion: reduce)')
}
