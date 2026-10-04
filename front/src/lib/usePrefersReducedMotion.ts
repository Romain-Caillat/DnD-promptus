import { useSyncExternalStore } from 'react'

const QUERY = '(prefers-reduced-motion: reduce)'

function subscribe(onChange: () => void) {
  const media = window.matchMedia(QUERY)
  media.addEventListener('change', onChange)
  return () => media.removeEventListener('change', onChange)
}

function getSnapshot() {
  return window.matchMedia(QUERY).matches
}

/**
 * Whether the device asks for reduced motion, kept live. CSS animations
 * are switched off globally in `styles/tokens.css`; motion driven from
 * JavaScript (a number that rolls, a timer that waits for an
 * `animationend`) must read this and jump straight to its calm, settled
 * state instead (MEMORY.md §4).
 */
export function usePrefersReducedMotion(): boolean {
  return useSyncExternalStore(subscribe, getSnapshot, () => false)
}
