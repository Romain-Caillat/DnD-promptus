import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { GmShops } from '@/lib/shop'
import { ShopsPanel } from './ShopsPanel'

const DATA: GmShops = {
  shops: [
    {
      id: 's1',
      name: 'Le marché noir',
      open: true,
      lines: [
        { id: 'compas', item: 'compas_enchante', name: '', description: '', price: 8, stock: 1, hidden: false },
        { id: 'vue', item: null, name: 'Longue-vue', description: '', price: 2, stock: null, hidden: true },
      ],
      haggle: { ability: 'CHA', difficulty: 12, discount: 25 },
      haggles: [{ line: 'compas', player: 'p1', success: true }],
    },
  ],
  items: [{ id: 'compas_enchante', name: 'Compas enchanté', price: 15 }],
  abilities: [
    ['FOR', 'Force'],
    ['CHA', 'Charisme'],
  ],
  currency: { name: "Pièces d'or", abbr: 'PO' },
}

function renderPanel(data = DATA) {
  const h = {
    onSave: vi.fn(async () => true),
    onOpen: vi.fn(),
    onReveal: vi.fn(),
    onDelete: vi.fn(),
  }
  render(<ShopsPanel data={data} {...h} />)
  return h
}

describe('ShopsPanel', () => {
  it('shows each shop, closes it, and brings a line out from under the counter', async () => {
    const h = renderPanel()
    const panel = screen.getByRole('region', { name: 'Boutiques' })
    expect(within(panel).getByText(/Compas enchanté · 8 PO · 1 en stock · marchandé : 1 réussi sur 1/)).toBeInTheDocument()
    await userEvent.click(within(panel).getByRole('button', { name: 'Fermer' }))
    expect(h.onOpen).toHaveBeenCalledWith('s1', false)
    await userEvent.click(within(panel).getByRole('button', { name: 'Sortir du comptoir · Longue-vue' }))
    expect(h.onReveal).toHaveBeenCalledWith('s1', 'vue')
  })

  it('writes a new shop: a rules item at its price, a named line under the counter, haggling', async () => {
    const h = renderPanel({ ...DATA, shops: [] })
    await userEvent.click(screen.getByRole('button', { name: 'Nouvelle boutique' }))
    await userEvent.type(screen.getByRole('textbox', { name: 'Nom de la boutique' }), 'Étal de Kerjean')
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter une ligne' }))
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Objet de la ligne 1' }), 'compas_enchante')
    await userEvent.click(screen.getByRole('button', { name: 'Ajouter une ligne' }))
    await userEvent.type(screen.getByRole('textbox', { name: 'Nom de l’objet, ligne 2' }), 'Rhum')
    await userEvent.click(screen.getAllByRole('checkbox', { name: 'Sous le comptoir' })[1])
    await userEvent.click(screen.getByRole('checkbox', { name: /La table peut marchander/ }))
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Caractéristique du marchandage' }), 'CHA')
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))
    expect(h.onSave).toHaveBeenCalledWith(null, {
      name: 'Étal de Kerjean',
      lines: [
        { item: 'compas_enchante', name: '', description: '', price: 15, stock: null, hidden: false },
        { item: null, name: 'Rhum', description: '', price: 1, stock: null, hidden: true },
      ],
      haggle: { ability: 'CHA', difficulty: 12, discount: 20 },
    })
    // Saved: back to the list.
    expect(await screen.findByRole('region', { name: 'Boutiques' })).toBeInTheDocument()
  })

  it('edits a shop keeping its line ids', async () => {
    const h = renderPanel()
    await userEvent.click(screen.getByRole('button', { name: 'Modifier' }))
    await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))
    const [id, input] = h.onSave.mock.calls[0] as unknown as [string, { lines: { id: string }[] }]
    expect(id).toBe('s1')
    expect(input.lines.map((l) => l.id)).toEqual(['compas', 'vue'])
  })
})
