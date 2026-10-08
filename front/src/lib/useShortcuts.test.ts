import { describe, expect, it } from 'vitest'
import { cardIndex, cardKey, shortcutKey } from './useShortcuts'

const key = (k: string, target: EventTarget | null = document.body, more: Partial<KeyboardEvent> = {}) =>
  shortcutKey({ key: k, target, repeat: false, altKey: false, ctrlKey: false, metaKey: false, ...more })

describe('shortcutKey', () => {
  it('hears digits, Space, Enter and Escape on the page', () => {
    expect(['1', '0', ' ', 'Enter', 'Escape', 'a'].map((k) => key(k))).toEqual(['1', '0', ' ', 'Enter', 'Escape', null])
  })

  it('leaves the keys to a field being typed in', () => {
    for (const tag of ['textarea', 'input', 'select']) {
      expect(key('3', document.createElement(tag))).toBeNull()
      expect(key(' ', document.createElement(tag))).toBeNull()
    }
  })

  it('lets a focused button take Space and Enter itself, so a roll never goes out twice', () => {
    const button = document.createElement('button')
    expect(key(' ', button)).toBeNull()
    expect(key('Enter', button)).toBeNull()
    // A card just clicked keeps the focus: the next digit still picks.
    expect(key('2', button)).toBe('2')
  })

  it('ignores a held key and the browser’s own shortcuts', () => {
    expect(key('1', document.body, { repeat: true })).toBeNull()
    expect(key('1', document.body, { metaKey: true })).toBeNull()
    expect(key('1', document.body, { ctrlKey: true })).toBeNull()
  })
})

describe('card keys', () => {
  it('numbers the first nine cards and leaves the tenth to the mouse', () => {
    expect([0, 8, 9].map(cardKey)).toEqual(['1', '9', null])
    expect(['1', '9', '0', 'x'].map(cardIndex)).toEqual([0, 8, null, null])
  })
})
