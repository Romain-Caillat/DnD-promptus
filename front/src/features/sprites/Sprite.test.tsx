import { render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { mockApi } from '@/test-utils'
import { spriteUrl, type CharacterLook } from './look'
import { Sprite } from './Sprite'
import { SpritesSection } from './SpritesSection'

const BRETTEUR: CharacterLook = {
  pack: 'marins-1718',
  body: 'svelte',
  skin: 'hale',
  hair: { style: 'catogan', colour: 'brun' },
  outfit: { piece: 'gilet', dye: 'rouge' },
  weapon: 'sabre',
}

function lookOf(src: string) {
  const url = new URL(src, 'http://x')
  return { look: JSON.parse(url.searchParams.get('look') ?? 'null'), facing: url.searchParams.get('facing') }
}

describe('Sprite', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('loads the server render of its description, enlarged by a whole number', () => {
    render(<Sprite look={BRETTEUR} scale={3} facing="west" label="Le Bretteur" />)
    const img = screen.getByRole('img', { name: 'Le Bretteur' })
    expect(img).toHaveAttribute('width', String(22 * 3))
    expect(img).toHaveAttribute('height', String(28 * 3))
    const { look, facing } = lookOf(img.getAttribute('src') ?? '')
    expect(look).toEqual({ ...BRETTEUR, weapon: { piece: 'sabre' } })
    expect(facing).toBe('west')
  })

  it('gives one look one URL, however its keys were written', () => {
    const reordered = {
      weapon: { piece: 'sabre' },
      outfit: { dye: 'rouge', piece: 'gilet' },
      hair: { colour: 'brun', style: 'catogan' },
      skin: 'hale',
      body: 'svelte',
      pack: 'marins-1718',
    } as CharacterLook
    expect(spriteUrl(reordered)).toBe(spriteUrl(BRETTEUR))
    expect(spriteUrl({ ...BRETTEUR, skin: 'brun' })).not.toBe(spriteUrl(BRETTEUR))
  })

  it('draws each condition effect over the character', () => {
    const { container } = render(<Sprite look={BRETTEUR} effects={['etourdi', 'beni']} />)
    const root = container.firstElementChild
    expect(root).toHaveClass('sprite-fx-etourdi', 'sprite-fx-beni')
    // Three stars circling, four sparkles and a halo.
    expect(container.querySelectorAll('.sprite-ring .sprite-particle-etourdi')).toHaveLength(3)
    expect(container.querySelectorAll('.sprite-particle-beni')).toHaveLength(4)
    expect(container.querySelector('.sprite-halo')).not.toBeNull()
    // A decorative sprite stays out of the accessibility tree.
    expect(screen.queryByRole('img')).toBeNull()
  })
})

describe('SpritesSection', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('shows the party facing right and the foes facing left', async () => {
    mockApi({
      'GET /api/sprites/looks': () => ({
        status: 200,
        body: {
          data: {
            worlds: [
              {
                world: 'corsaires',
                party: [{ id: 'pj_bretteur', look: BRETTEUR }],
                foes: [{ id: 'gueule-rouge', look: { ...BRETTEUR, body: 'robuste' } }],
              },
            ],
          },
        },
      }),
    })

    render(<SpritesSection />)

    expect(await screen.findByText('Corsaires de la Couronne')).toBeInTheDocument()
    expect(lookOf(screen.getByRole('img', { name: 'pj_bretteur' }).getAttribute('src') ?? '').facing).toBe('east')
    expect(lookOf(screen.getByRole('img', { name: 'gueule-rouge' }).getAttribute('src') ?? '').facing).toBe('west')
    expect(screen.getByText('Empoisonné')).toBeInTheDocument()
  })
})
