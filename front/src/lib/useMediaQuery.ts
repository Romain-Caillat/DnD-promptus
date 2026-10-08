import { useCallback, useSyncExternalStore } from 'react'

/**
 * Whether a CSS media query matches, kept live. Layout choices that
 * decide what is mounted (and so what is fetched) read this instead of
 * hiding with CSS: a phone never mounts the desktop columns.
 */
export function useMediaQuery(query: string): boolean {
  const subscribe = useCallback(
    (onChange: () => void) => {
      const media = window.matchMedia(query)
      media.addEventListener('change', onChange)
      return () => media.removeEventListener('change', onChange)
    },
    [query],
  )
  return useSyncExternalStore(
    subscribe,
    () => window.matchMedia(query).matches,
    () => false,
  )
}

/**
 * A computer: a wide screen and a fine pointer (player/play-on-desktop).
 * A tablet keeps the phone layout on the player side: its player has no
 * keyboard for the shortcuts.
 */
export const DESKTOP_QUERY = '(min-width: 1100px) and (pointer: fine)'

/** A tablet: a coarse pointer on a screen wider than a phone (gm/run-on-tablet). */
export const TABLET_QUERY = '(pointer: coarse) and (min-width: 768px)'
