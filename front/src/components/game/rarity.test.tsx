import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { GameCard } from './GameCard'
import { ITEM_SPRITES } from './itemSprites'
import { ItemSlot } from './ItemSlot'
import { RARITIES } from './rarity'

const NAMES = ['commune', 'peu commune', 'rare', 'épique', 'légendaire', 'divine']

// Rarity is read without colour: from 1 to 6 diamonds and a name.
describe('rarity without colour', () => {
  it.each(RARITIES.map((r, i) => [r, i + 1, NAMES[i]] as const))(
    'a %s card shows %i diamonds and names its tier',
    (rarity, pips, name) => {
      render(<GameCard kind="action" title="Frappe" rarity={rarity} deal={false} />)
      const mark = screen.getByRole('img', { name: `Rareté : ${name}` })
      expect(mark.querySelectorAll('i')).toHaveLength(pips)
    },
  )

  it.each(RARITIES.map((r, i) => [r, i + 1, NAMES[i]] as const))(
    'a %s item slot shows %i diamonds and names its tier',
    (rarity, pips, name) => {
      const { container } = render(
        <ItemSlot sprite={ITEM_SPRITES.axe} name="Hache" rarity={rarity} quantity={2} />,
      )
      expect(screen.getByRole('img', { name: `Hache, ${name}, quantité 2` })).toBeInTheDocument()
      expect(container.querySelectorAll('.gk-pips i')).toHaveLength(pips)
    },
  )

  it('a card without rarity shows no diamonds', () => {
    render(<GameCard kind="clue" title="La page arrachée" deal={false} />)
    expect(screen.queryByRole('img', { name: /Rareté/ })).toBeNull()
    expect(screen.getByText('Indice')).toBeInTheDocument()
  })
})
