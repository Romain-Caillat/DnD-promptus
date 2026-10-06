import { describe, expect, it } from 'vitest'
import { formatDollars, parseDollars } from './campaigns'

describe('AI budget in dollars', () => {
  it('reads what a French GM types, in cents', () => {
    expect(parseDollars('10')).toBe(1000)
    expect(parseDollars('2,50')).toBe(250)
    expect(parseDollars('2,5')).toBe(250)
    expect(parseDollars(' 0.05 ')).toBe(5)
  })

  it('refuses what is not an amount', () => {
    for (const text of ['', '-1', '1,234', 'dix', '1 000']) expect(parseDollars(text)).toBeNull()
  })

  it('writes cents back the way they are typed', () => {
    for (const cents of [0, 5, 250, 1000, 1234]) expect(parseDollars(formatDollars(cents))).toBe(cents)
    expect(formatDollars(250)).toBe('2,50')
  })
})
