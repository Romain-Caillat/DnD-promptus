import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { PackCatalogue } from '@/lib/creator'
import { AppearanceStep } from './AppearanceStep'

const PACK: PackCatalogue = {
  id: 'equipage-spatial',
  name: 'Équipage spatial',
  slots: {
    body: [{ id: 'svelte', name: 'Svelte', dyed: false, accented: false }],
    outfit: [],
    armour: [],
    hair: [],
    beard: [],
    headwear: [],
    accessory: [],
    weapon: [],
  },
  palettes: { skin: [{ id: 'clair', name: 'Clair', colour: '#F0C09A' }], hair: [], cloth: [] },
}
const LOOK = { pack: 'equipage-spatial', body: 'svelte', skin: 'clair', hair: { colour: 'roux' } }

const facingOf = (el: HTMLElement) =>
  new URL(/url\("(.+)"\)/.exec(el.style.backgroundImage)?.[1] ?? '', 'http://x').searchParams.get('facing')

describe('AppearanceStep', () => {
  it('turns the preview to the four ways the character walks, and makes it walk', async () => {
    render(
      <AppearanceStep pack={PACK} look={LOOK} onLook={() => {}} tab="body" onTab={() => {}} name="Ash" onName={() => {}} />,
    )
    const preview = () => screen.getByRole('img', { name: 'Ash' })
    expect(facingOf(preview())).toBe('east')
    expect(preview()).toHaveClass('sprite-sheet-repos')
    await userEvent.click(screen.getByRole('button', { name: 'De dos' }))
    expect(facingOf(preview())).toBe('north')
    expect(screen.getByRole('button', { name: 'De dos' })).toHaveAttribute('aria-pressed', 'true')
    await userEvent.click(screen.getByRole('button', { name: 'De face' }))
    expect(facingOf(preview())).toBe('south')
    await userEvent.click(screen.getByRole('button', { name: 'Marcher' }))
    expect(preview()).toHaveClass('sprite-sheet-marche')
  })
})
