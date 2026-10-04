import { describe, expect, it } from 'vitest'
import { cn } from './utils'

describe('cn', () => {
  it('keeps a token text size next to a text colour', () => {
    expect(cn('text-label text-chalk')).toBe('text-label text-chalk')
  })

  it('still lets a later token size override an earlier one', () => {
    expect(cn('text-label', 'text-heading')).toBe('text-heading')
  })
})
