import { useSyncExternalStore } from 'react'

/**
 * The theme setting (ui/offer-light-theme), kept per device: the black
 * table by default, a light table, or whatever the device prefers.
 *
 * index.html applies the stored choice before the first paint with the
 * same key and rules; `startTheme` (main.tsx) then keeps `<html>` in step
 * with the device when the choice is « Comme l'appareil ».
 */
export type ThemeChoice = 'dark' | 'light' | 'system'
export const THEME_CHOICES: readonly ThemeChoice[] = ['dark', 'light', 'system']

export const THEME_KEY = 'promptus.theme'
const LIGHT_QUERY = '(prefers-color-scheme: light)'

const listeners = new Set<() => void>()
// Fallback when storage throws, so the setting still answers in the session.
let memory: ThemeChoice | null = null

function isChoice(v: unknown): v is ThemeChoice {
  return v === 'dark' || v === 'light' || v === 'system'
}

/** The stored choice; dark when none or when storage is unavailable. */
function readThemeChoice(): ThemeChoice {
  try {
    const v = localStorage.getItem(THEME_KEY)
    return isChoice(v) ? v : 'dark'
  } catch {
    return 'dark'
  }
}

function deviceLight(): boolean {
  return typeof window.matchMedia === 'function' && window.matchMedia(LIGHT_QUERY).matches
}

/** The theme actually drawn for a choice. */
function resolveTheme(choice: ThemeChoice): 'dark' | 'light' {
  if (choice === 'system') return deviceLight() ? 'light' : 'dark'
  return choice
}

/** Set `<html>`: `data-theme` for the tokens, `.dark` for shadcn's `dark:` variants. */
function applyTheme(choice: ThemeChoice) {
  const theme = resolveTheme(choice)
  const root = document.documentElement
  root.dataset.theme = theme
  root.classList.toggle('dark', theme === 'dark')
}

export function setThemeChoice(choice: ThemeChoice) {
  try {
    localStorage.setItem(THEME_KEY, choice)
    memory = null
  } catch {
    // Private mode or blocked storage: the choice holds until the page closes.
    memory = choice
  }
  applyTheme(choice)
  listeners.forEach((l) => l())
}

function currentChoice(): ThemeChoice {
  return memory ?? readThemeChoice()
}

/**
 * Apply the choice now and follow the device (« Comme l'appareil ») and
 * other tabs from then on. Returns the cleanup, for tests.
 */
export function startTheme(): () => void {
  applyTheme(currentChoice())
  const refresh = () => {
    applyTheme(currentChoice())
    listeners.forEach((l) => l())
  }
  const media = typeof window.matchMedia === 'function' ? window.matchMedia(LIGHT_QUERY) : null
  const onDevice = () => {
    if (currentChoice() === 'system') refresh()
  }
  const onStorage = (e: StorageEvent) => {
    if (e.key === THEME_KEY) refresh()
  }
  media?.addEventListener('change', onDevice)
  window.addEventListener('storage', onStorage)
  return () => {
    media?.removeEventListener('change', onDevice)
    window.removeEventListener('storage', onStorage)
  }
}

function subscribe(onChange: () => void) {
  listeners.add(onChange)
  return () => {
    listeners.delete(onChange)
  }
}

/** The stored choice, kept live. */
export function useThemeChoice(): ThemeChoice {
  return useSyncExternalStore(subscribe, currentChoice, () => 'dark')
}

/** The theme drawn right now (`<html data-theme>`), kept live: the canvas redraws on change. */
export function useResolvedTheme(): 'dark' | 'light' {
  return useSyncExternalStore(
    subscribe,
    () => (document.documentElement.dataset.theme === 'light' ? 'light' : 'dark'),
    () => 'dark',
  )
}
