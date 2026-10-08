import { act, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { THEME_KEY, startTheme } from '@/lib/theme'
import { ThemeSetting } from './ThemeSetting'

// A device whose light preference the test flips, telling its listeners.
function fakeDevice(light: boolean) {
  const listeners = new Set<() => void>()
  const media = {
    get matches() {
      return light
    },
    addEventListener: (_: string, l: () => void) => listeners.add(l),
    removeEventListener: (_: string, l: () => void) => listeners.delete(l),
  }
  vi.stubGlobal('matchMedia', () => media)
  return (next: boolean) => {
    light = next
    listeners.forEach((l) => l())
  }
}

function fakeStorage(initial: Record<string, string> = {}) {
  const items = new Map(Object.entries(initial))
  vi.stubGlobal('localStorage', {
    getItem: (k: string) => items.get(k) ?? null,
    setItem: (k: string, v: string) => void items.set(k, v),
    removeItem: (k: string) => void items.delete(k),
  })
  return items
}

const html = () => document.documentElement

describe('ThemeSetting', () => {
  let stop: () => void = () => {}

  beforeEach(() => {
    html().removeAttribute('data-theme')
    html().classList.add('dark')
  })
  afterEach(() => {
    stop()
    vi.unstubAllGlobals()
  })

  it('is the black table by default', () => {
    fakeStorage()
    fakeDevice(true)
    stop = startTheme()
    render(<ThemeSetting />)
    expect(screen.getByRole('radio', { name: 'Sombre' })).toHaveAttribute('aria-checked', 'true')
    expect(html().dataset.theme).toBe('dark')
    expect(html()).toHaveClass('dark')
  })

  it('keeps the choice on the device and applies it to the page', async () => {
    const items = fakeStorage()
    fakeDevice(false)
    stop = startTheme()
    render(<ThemeSetting />)
    await userEvent.click(screen.getByRole('radio', { name: 'Clair' }))
    expect(items.get(THEME_KEY)).toBe('light')
    expect(html().dataset.theme).toBe('light')
    expect(html()).not.toHaveClass('dark')
    expect(screen.getByRole('radio', { name: 'Clair' })).toHaveAttribute('aria-checked', 'true')

    await userEvent.click(screen.getByRole('radio', { name: 'Sombre' }))
    expect(items.get(THEME_KEY)).toBe('dark')
    expect(html().dataset.theme).toBe('dark')
    expect(html()).toHaveClass('dark')
  })

  it('follows the device live when set to « Comme l’appareil »', async () => {
    fakeStorage({ [THEME_KEY]: 'system' })
    const setDevice = fakeDevice(false)
    stop = startTheme()
    render(<ThemeSetting />)
    expect(screen.getByRole('radio', { name: 'Comme l’appareil' })).toHaveAttribute('aria-checked', 'true')
    expect(html().dataset.theme).toBe('dark')

    act(() => setDevice(true))
    expect(html().dataset.theme).toBe('light')
    act(() => setDevice(false))
    expect(html().dataset.theme).toBe('dark')
  })

  it('ignores the device once a theme is chosen', () => {
    fakeStorage({ [THEME_KEY]: 'dark' })
    const setDevice = fakeDevice(false)
    stop = startTheme()
    act(() => setDevice(true))
    expect(html().dataset.theme).toBe('dark')
  })
})
