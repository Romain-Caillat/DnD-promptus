// Draft for front/src/features/play/creator/CharacterCreatorPage.test.tsx (not yet run).
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { CharacterSheet } from '@/lib/play'
import { mockApi, sentTo } from '@/test-utils'
import { CharacterCreatorPage } from './CharacterCreatorPage'

function renderCreator() {
  render(
    <MemoryRouter initialEntries={['/partie/c1/personnage']}>
      <Routes>
        <Route path="/partie/:campaignId/personnage" element={<CharacterCreatorPage />} />
        <Route path="/partie/:campaignId" element={<p>accueil joueur</p>} />
      </Routes>
    </MemoryRouter>,
  )
}

const CAMPAIGN = { campaignId: 'c1', title: 'Les Cendres de Valombre', world: 'Valombre', playerHook: null, gmName: 'Romain' }
const START_LOOK = { pack: 'marins-1718', body: 'robuste', skin: 'hale', hair: { style: 'court', colour: 'roux' } }
const PACK = {
  id: 'marins-1718',
  name: 'Marins 1718',
  slots: {
    body: [{ id: 'robuste', name: 'Robuste', dyed: false, accented: false }],
    outfit: [{ id: 'chemise', name: 'Chemise', dyed: true, accented: false }],
    armour: [],
    hair: [{ id: 'court', name: 'Court', dyed: false, accented: false }],
    beard: [{ id: 'longue', name: 'Longue', dyed: false, accented: false }],
    headwear: [{ id: 'tricorne', name: 'Tricorne', dyed: true, accented: true }],
    accessory: [],
    weapon: [{ id: 'hache', name: 'Hache', dyed: false, accented: false }],
  },
  palettes: {
    skin: [{ id: 'hale', name: 'Hâlée', colour: '#C68A5A' }],
    hair: [
      { id: 'roux', name: 'Roux', colour: '#B5502A' },
      { id: 'noir', name: 'Noir', colour: '#1A1A1A' },
    ],
    cloth: [
      { id: 'rouge', name: 'Rouge', colour: '#A33' },
      { id: 'bleu', name: 'Bleu', colour: '#33A' },
    ],
  },
}
const STATS = {
  hitPoints: 13, armorClass: 16, initiative: 0, modifiers: { FOR: 2, DEX: 0 },
  cards: [{ id: 'hache', name: 'Hache d’armes', description: 'Un coup lourd.', kind: 'Attaque', level: 1, attackBonus: 4, damage: '1d12', heal: null, cooldown: 0, range: 1 }],
}
const CREATION = {
  pack: 'marins-1718',
  startLook: START_LOOK,
  rules: {
    name: 'Valombre',
    abilities: [
      { id: 'FOR', name: 'Force', description: 'Frapper, porter.' },
      { id: 'DEX', name: 'Dextérité', description: 'Esquiver.' },
    ],
    peoples: [
      { id: 'nain', name: 'Nain', description: 'Tient debout quand tout tombe.' },
      { id: 'elfe', name: 'Elfe', description: 'Voit loin.' },
    ],
    classes: [{ id: 'guerrier', name: 'Guerrier', description: 'Frapper fort, protéger les autres.', primaryAbilities: ['FOR'], abilities: { FOR: 17, DEX: 10 }, budget: 27, stats: STATS }],
  },
}

function character(status: string, sheet: CharacterSheet, gmNote: string | null = null) {
  return { id: 'k1', status, sheet, gmNote, updatedAt: '', peopleName: sheet.peopleId ? 'Nain' : null, className: sheet.classId ? 'Guerrier' : null, stats: sheet.classId ? STATS : null }
}
function home(status: string, sheet: CharacterSheet = {}, gmNote: string | null = null) {
  return { me: { id: 'p1', nickname: 'Marc', role: 'player' }, campaign: CAMPAIGN, character: character(status, sheet, gmNote) }
}
function server(start: ReturnType<typeof home>) {
  let sheet: CharacterSheet = start.character.sheet
  return mockApi({
    'GET /api/play/c1/me': () => ({ status: 200, body: { data: start } }),
    'GET /api/play/c1/creation': () => ({ status: 200, body: { data: CREATION } }),
    'GET /api/sprites/packs/marins-1718': () => ({ status: 200, body: { data: PACK } }),
    'PUT /api/play/c1/character': (body) => {
      sheet = body as CharacterSheet
      return { status: 200, body: { data: character(start.character.status, sheet) } }
    },
    'POST /api/play/c1/character/backstory': (body) => {
      const { origin } = body as { origin: string }
      return { status: 200, body: { data: { text: `Borin vient de ${origin.toLowerCase()}.` } } }
    },
    'POST /api/play/c1/character/submit': () => ({ status: 200, body: { data: character('submitted', sheet) } }),
  })
}
const next = () => userEvent.click(screen.getByRole('button', { name: /^Continuer/ }))

describe('CharacterCreatorPage', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('walks Marc through the eight moments, flags the limit he passes, and sends Borin to the GM', async () => {
    const api = server(home('draft'))
    renderCreator()
    expect(await screen.findByRole('heading', { name: 'Ton peuple' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /^Continuer/ })).toBeDisabled()
    await userEvent.click(screen.getByRole('button', { name: /Nain/ }))
    await next()
    expect(await screen.findByRole('heading', { name: 'Ton allure' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Longue' }))
    await userEvent.click(screen.getByRole('button', { name: /Au hasard/ }))
    await next()
    expect(await screen.findByRole('heading', { name: 'Tenue et arme' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Tricorne' }))
    await userEvent.click(screen.getByRole('button', { name: 'Chemise' }))
    await userEvent.click(screen.getByRole('button', { name: 'Hache' }))
    await next()
    expect(await screen.findByRole('heading', { name: 'Couleurs et nom' })).toBeInTheDocument()
    await userEvent.type(screen.getByLabelText('Son nom'), 'Borin')
    await next()
    expect(await screen.findByRole('heading', { name: 'Ce que tu sais faire' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Guerrier/ }))
    await next()
    expect(await screen.findByRole('heading', { name: 'Tes caractéristiques' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Monter Force' }))
    expect(screen.getByRole('note')).toHaveTextContent('1 point au-delà de la répartition')
    expect(screen.getByRole('button', { name: /^Continuer/ })).toBeEnabled()
    await next()
    expect(await screen.findByRole('heading', { name: 'Son histoire' })).toBeInTheDocument()
    const write = screen.getByRole('button', { name: /Écrire avec le co-MJ/ })
    expect(write).toBeDisabled()
    await userEvent.type(screen.getByLabelText("D'où vient-il ?"), 'La mine de Valombre')
    await userEvent.click(write)
    // The co-GM's paragraph is the player's to keep or edit.
    const text = screen.getByLabelText('Son histoire en quelques lignes (facultatif)')
    expect(text).toHaveValue('Borin vient de la mine de valombre.')
    expect(sentTo(api, 'POST /api/play/c1/character/backstory')).toEqual([
      { origin: 'La mine de Valombre', loss: '', quest: '' },
    ])
    await next()
    expect(await screen.findByRole('heading', { name: 'Relire et envoyer' })).toBeInTheDocument()
    expect(screen.getByText('Nain · Guerrier · niveau 1')).toBeInTheDocument()
    expect(screen.getByRole('note')).toHaveTextContent('1 point à voir avec le MJ')
    await userEvent.click(screen.getByRole('button', { name: /Envoyer au MJ/ }))
    expect(await screen.findByText('Romain regarde ton personnage…')).toBeInTheDocument()
    const saved = sentTo(api, 'PUT /api/play/c1/character').at(-1) as CharacterSheet
    expect(saved).toMatchObject({ name: 'Borin', peopleId: 'nain', classId: 'guerrier', abilities: { FOR: 18, DEX: 10 }, backstory: { origin: 'La mine de Valombre', text: 'Borin vient de la mine de valombre.' } })
    expect(saved.look?.pack).toBe('marins-1718')
  })

  it('does not reopen a character the GM is reading', async () => {
    server(home('submitted', { name: 'Borin' }))
    renderCreator()
    expect(await screen.findByText('accueil joueur')).toBeInTheDocument()
  })
})
