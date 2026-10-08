import { useEffect, useRef } from 'react'

/**
 * A key the shortcuts answer: a digit `'0'`–`'9'`, `' '` (Space),
 * `'Enter'` or `'Escape'`. Return `true` when the key did something, so
 * the page does not also scroll or submit.
 */
export type ShortcutHandler = (key: string) => boolean

function typing(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  return target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)
}

/**
 * Should `e` reach the shortcuts? Never while typing in a field, never
 * with a modifier held (the browser's own shortcuts), never a held key
 * repeating; Space and Enter not on a focused button or link, which
 * already acts on them — a roll would go out twice.
 */
export function shortcutKey(e: Pick<KeyboardEvent, 'key' | 'target' | 'repeat' | 'altKey' | 'ctrlKey' | 'metaKey'>): string | null {
  if (e.repeat || e.altKey || e.ctrlKey || e.metaKey || typing(e.target)) return null
  if (/^[0-9]$/.test(e.key) || e.key === 'Escape') return e.key
  if (e.key === ' ' || e.key === 'Enter') {
    const el = e.target instanceof HTMLElement ? e.target : null
    if (el && (el.tagName === 'BUTTON' || el.tagName === 'A' || el.getAttribute('role') === 'button')) return null
    return e.key
  }
  return null
}

/**
 * The keyboard of player/play-on-desktop: `handler` hears the keys of
 * `shortcutKey` while `enabled`. Several parts of a screen may listen;
 * each enables itself only when the keys are its own (the fight hand on
 * my turn, the scene hand otherwise).
 */
export function useShortcuts(enabled: boolean, handler: ShortcutHandler) {
  const ref = useRef(handler)
  useEffect(() => {
    ref.current = handler
  })
  useEffect(() => {
    if (!enabled) return
    const onKey = (e: KeyboardEvent) => {
      const key = shortcutKey(e)
      if (key !== null && ref.current(key)) e.preventDefault()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [enabled])
}

/**
 * The key of the `index`-th card of a hand: 1 to 9, nothing past the
 * ninth (a click still plays it). « Autre… » is always 0.
 */
export function cardKey(index: number): string | null {
  return index < 9 ? String(index + 1) : null
}

/** The card index a digit picks, or `null` (0 is « Autre… »). */
export function cardIndex(key: string): number | null {
  return /^[1-9]$/.test(key) ? Number(key) - 1 : null
}
