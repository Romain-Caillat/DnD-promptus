/**
 * The tools as Claude calls them: a real MCP client over an in-memory
 * transport, the Promptus API stubbed by a mocked `fetch`.
 */
import { afterEach, describe, expect, it } from 'bun:test'
import { Client } from '@modelcontextprotocol/sdk/client/index.js'
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js'
import { createServer } from './server'

const ENV = { PROMPTUS_URL: 'http://promptus.test/', PROMPTUS_TOKEN: 'promptus_aaaaaaaa' }

type Handler = (body: unknown) => { status: number; body?: unknown }
interface Sent {
  key: string
  auth: string | null
  body: unknown
}

/** A `fetch` answering `"METHOD path"` (path after `/api`), recording every call. */
function mockFetch(routes: Record<string, Handler>) {
  const sent: Sent[] = []
  const fetchImpl = (async (input: string | URL | Request, init?: RequestInit) => {
    const url = new URL(String(input))
    expect(url.origin).toBe('http://promptus.test')
    const key = `${init?.method ?? 'GET'} ${url.pathname.replace(/^\/api/, '')}`
    const headers = new Headers(init?.headers)
    const body = typeof init?.body === 'string' ? JSON.parse(init.body) : undefined
    sent.push({ key, auth: headers.get('Authorization'), body })
    const handler = routes[key]
    if (!handler) throw new Error(`unexpected call ${key}`)
    const answer = handler(body)
    return new Response(answer.status === 204 ? null : JSON.stringify(answer.body ?? null), {
      status: answer.status,
      headers: { 'Content-Type': 'application/json' },
    })
  }) as typeof fetch
  return { fetchImpl, sent }
}

let client: Client | null = null

async function connect(env: Record<string, string | undefined>, fetchImpl: typeof fetch) {
  const server = createServer(env, fetchImpl)
  const [clientSide, serverSide] = InMemoryTransport.createLinkedPair()
  await server.connect(serverSide)
  client = new Client({ name: 'test', version: '0' })
  await client.connect(clientSide)
  return client
}

afterEach(async () => {
  await client?.close()
  client = null
})

async function call(c: Client, name: string, args: Record<string, unknown> = {}) {
  const result = await c.callTool({ name, arguments: args })
  const content = result.content as { type: string; text: string }[]
  return { isError: result.isError === true, text: content[0]?.text ?? '' }
}

const STORY = {
  id: 'phare',
  title: 'Le Phare de Kerbrume',
  world: 'Bretagne, 1890',
  rules: { id: 'corsaires', version: 1 },
  bible: { pitch: 'Un phare éteint.', tone: 'Brume', player_hook: 'Le gardien a disparu.', start_node: 'sc_auberge' },
  acts: [{ id: 'acte_1', title: 'La brume', summary: 'On arrive.', gm_notes: 'secret long' }],
  nodes: [
    {
      id: 'sc_auberge',
      act: 'acte_1',
      title: "L'auberge",
      summary: 'Les pêcheurs se taisent.',
      read_aloud: 'La porte grince.',
      npcs: [{ npc: 'pnj_morel', role: 'aubergiste' }],
      exits: [{ to: 'sc_quai', label: 'Le quai' }],
    },
    { id: 'sc_quai', act: 'acte_1', title: 'Le quai', summary: 'Des filets.' },
  ],
  clues: [
    { id: 'cl_filet', revelation: 'rv_gardien', node: 'sc_auberge', text: 'Un filet déchiré.' },
    { id: 'cl_lanterne', revelation: 'rv_gardien', node: 'sc_quai', text: 'Une lanterne.' },
  ],
  npcs: [{ id: 'pnj_morel', name: 'Morel', title: 'aubergiste', wants: 'Partir.' }],
}

const DETAIL = {
  id: 'c1',
  story: STORY,
  issues: [
    { severity: 'warning', code: 'THREE_CLUE_RULE', path: 'revelations.rv_gardien', detail: 'rv_gardien: 2 nodes' },
    { severity: 'error', code: 'REF_DANGLING', path: 'nodes.sc_auberge.location', detail: 'sc_auberge: no such location' },
  ],
  validatedAt: null,
  archivedAt: null,
  updatedAt: '2026-10-08T10:00:00Z',
}

describe('promptus MCP server', () => {
  it('offers the preparation tools, described in French', async () => {
    const c = await connect(ENV, mockFetch({}).fetchImpl)
    const { tools } = await c.listTools()
    expect(tools.map((t) => t.name).sort()).toEqual([
      'apply_story_edits',
      'ask_map',
      'create_campaign_from_yaml',
      'edit_map',
      'export_campaign_yaml',
      'get_atlas',
      'get_campaign',
      'get_map_text',
      'get_readiness',
      'get_scene',
      'get_theme_needs',
      'import_campaign_yaml',
      'import_pixellab_asset',
      'list_campaigns',
      'list_proposals',
      'list_rule_systems',
      'place_on_map',
      'propose_scene',
      'propose_tension',
      'start_campaign',
    ])
    const edits = tools.find((t) => t.name === 'apply_story_edits')!
    expect(edits.description).toContain('tout ou rien')
    expect(edits.description).toContain('list_proposals')
    expect(edits.inputSchema.required).toEqual(['campaign_id', 'edits'])
  })

  it('lists the campaigns with the bearer token', async () => {
    const api = mockFetch({
      'GET /campaigns': () => ({
        status: 200,
        body: { data: [{ id: 'c1', title: 'Le Phare', world: 'Bretagne', archivedAt: null, playersSeated: 3 }] },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'list_campaigns')
    expect(r.isError).toBe(false)
    expect(JSON.parse(r.text)).toEqual([{ id: 'c1', title: 'Le Phare', world: 'Bretagne', archived: false }])
    expect(api.sent[0]!.auth).toBe('Bearer promptus_aaaaaaaa')
  })

  it('summarizes a campaign without its long texts, issues errors first', async () => {
    const api = mockFetch({ 'GET /campaigns/c1': () => ({ status: 200, body: { data: DETAIL } }) })
    const c = await connect(ENV, api.fetchImpl)
    const summary = JSON.parse((await call(c, 'get_campaign', { campaign_id: 'c1' })).text)
    expect(summary.title).toBe('Le Phare de Kerbrume')
    expect(summary.scenes[0]).toEqual({
      id: 'sc_auberge',
      act: 'acte_1',
      title: "L'auberge",
      summary: 'Les pêcheurs se taisent.',
      exits: ['sc_quai'],
    })
    expect(summary.acts[0].gm_notes).toBeUndefined()
    expect(summary.issues.errors).toBe(1)
    expect(summary.issues.list[0].code).toBe('REF_DANGLING')
  })

  it('reads one scene with its clues, its NPCs and its alerts', async () => {
    const api = mockFetch({ 'GET /campaigns/c1': () => ({ status: 200, body: { data: DETAIL } }) })
    const c = await connect(ENV, api.fetchImpl)
    const found = JSON.parse((await call(c, 'get_scene', { campaign_id: 'c1', scene_id: 'sc_auberge' })).text)
    expect(found.scene.read_aloud).toBe('La porte grince.')
    expect(found.clues.map((cl: { id: string }) => cl.id)).toEqual(['cl_filet'])
    expect(found.npcs).toEqual([{ npc: 'pnj_morel', role: 'aubergiste', name: 'Morel', title: 'aubergiste', wants: 'Partir.' }])
    expect(found.issues.map((i: { code: string }) => i.code)).toEqual(['REF_DANGLING'])

    const missing = await call(c, 'get_scene', { campaign_id: 'c1', scene_id: 'sc_phare' })
    expect(missing.isError).toBe(true)
    expect(missing.text).toContain('Pas de scène « sc_phare »')
    expect(missing.text).toContain("sc_auberge (L'auberge)")
  })

  it('gives the coherence alerts and the act gauges', async () => {
    const api = mockFetch({
      'GET /campaigns/c1': () => ({ status: 200, body: { data: DETAIL } }),
      'GET /campaigns/c1/readiness': () => ({
        status: 200,
        body: { data: [{ act: 'acte_1', title: 'La brume', ready: false, done: 3, total: 5, checks: [] }] },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = JSON.parse((await call(c, 'get_readiness', { campaign_id: 'c1' })).text)
    expect(r.issues).toMatchObject({ errors: 1, warnings: 1 })
    expect(r.acts[0].done).toBe(3)
    expect(r.playable.verdict).toBe('Pas encore jouable')
    expect(r.playable.missing).toEqual(['1 erreur(s) du validateur', "la jauge de « La brume » n'est pas pleine"])
  })

  it('passes the edits to the server as they are, and reports what changed', async () => {
    const edit = { op: 'add', kind: 'clue', value: { id: 'cl_quai', revelation: 'rv_gardien', node: 'sc_auberge', text: 'Un indice vers le quai.' } }
    const api = mockFetch({
      'POST /campaigns/c1/story/edits': () => ({
        status: 200,
        body: { data: { status: 'applied', campaign: { ...DETAIL, issues: [] }, changes: [{ kind: 'added', id: 'cl_quai' }] } },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'apply_story_edits', { campaign_id: 'c1', edits: [edit] })
    expect(r.isError).toBe(false)
    expect(api.sent[0]!.body).toEqual({ edits: [edit] })
    expect(JSON.parse(r.text)).toEqual({
      status: 'appliqué',
      changes: [{ kind: 'added', id: 'cl_quai' }],
      issues: { errors: 0, warnings: 0, list: [] },
    })
  })

  it('proposes the edits in a titled batch the GM decides on', async () => {
    const edit = { op: 'set', target: 'pnj_morel', field: 'name', value: 'Morel le Borgne' }
    const api = mockFetch({
      'POST /campaigns/c1/story/edits': () => ({
        status: 202,
        body: {
          data: {
            status: 'proposed',
            why: 'proposals',
            proposal: {
              id: 'p1',
              prompt: 'Peupler le cimetière',
              status: 'pending',
              items: [{ key: 'e0', status: 'pending', stale: false, edit }],
              changes: [{ op: 'set', kind: 'npc', id: 'pnj_morel' }],
              createdAt: '2026-10-11T10:00:00Z',
            },
            issues: [],
          },
        },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'apply_story_edits', {
      campaign_id: 'c1',
      edits: [edit],
      title: 'Peupler le cimetière',
      proposal_id: 'p0',
    })
    expect(r.isError).toBe(false)
    expect(api.sent[0]!.body).toEqual({ edits: [edit], title: 'Peupler le cimetière', proposal: 'p0' })
    const out = JSON.parse(r.text)
    expect(out.status).toBe('proposé')
    expect(out.proposal).toEqual({ id: 'p1', title: 'Peupler le cimetière', items: 1 })
    expect(out.why).toContain('le MJ garde la main')
    expect(out.next).toContain('list_proposals')
  })

  it("reads back what the GM kept, and the word on what was thrown away", async () => {
    const api = mockFetch({
      'GET /campaigns/c1/proposals': () => ({
        status: 200,
        body: {
          data: [
            {
              id: 'p1',
              prompt: 'Peupler le cimetière',
              status: 'pending',
              changes: [],
              createdAt: '2026-10-11T10:00:00Z',
              items: [
                { key: 'e0', status: 'kept', stale: false, edit: { op: 'add', kind: 'npc' } },
                { key: 'e1', status: 'discarded', stale: false, word: 'trop évident', edit: { op: 'add', kind: 'clue' } },
                { key: 'e2', status: 'refused', stale: false, code: 'EDIT_UNKNOWN_TARGET' },
                { key: 'm0', status: 'pending', stale: true, map: { map: 'crypte', name: 'La crypte', ops: 2 } },
              ],
            },
          ],
        },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'list_proposals', { campaign_id: 'c1' })
    expect(r.isError).toBe(false)
    const [batch] = JSON.parse(r.text)
    expect(batch.title).toBe('Peupler le cimetière')
    expect(batch.items[0].status).toBe('gardé')
    expect(batch.items[1]).toMatchObject({ status: 'jeté', word: 'trop évident' })
    expect(batch.items[2].refused).toBe('EDIT_UNKNOWN_TARGET')
    expect(batch.items[3].map).toBe('La crypte (2 opération(s))')
    expect(batch.items[3].stale).toBeDefined()
    expect(api.sent[0]!.auth).toBe('Bearer promptus_aaaaaaaa')
  })

  it('says a whole file needs the hand', async () => {
    const api = mockFetch({
      'PUT /campaigns/c1/import': () => ({ status: 409, body: { error: { code: 'CLAUDE_PROPOSES', message: '' } } }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'import_campaign_yaml', { campaign_id: 'c1', yaml: 'id: phare\n' })
    expect(r.isError).toBe(true)
    expect(r.text).toContain('Claude a la main')
    expect(r.text).toContain('apply_story_edits')
  })

  it("explains a refused edit with the server's reason", async () => {
    const api = mockFetch({
      'POST /campaigns/c1/story/edits': () => ({
        status: 400,
        body: { error: { code: 'EDIT_UNKNOWN_TARGET', message: 'edit 0: no such id sc_nulle' } },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'apply_story_edits', { campaign_id: 'c1', edits: [{ op: 'remove', target: 'sc_nulle' }] })
    expect(r.isError).toBe(true)
    expect(r.text).toContain("rien n'a été appliqué")
    expect(r.text).toContain('edit 0: no such id sc_nulle')
  })

  it('exports, then imports a YAML over an existing campaign', async () => {
    const api = mockFetch({
      'GET /campaigns/c1/export': () => ({ status: 200, body: { data: { yaml: 'id: phare\n' } } }),
      'PUT /campaigns/c1/import': () => ({ status: 200, body: { data: DETAIL } }),
    })
    const c = await connect(ENV, api.fetchImpl)
    expect((await call(c, 'export_campaign_yaml', { campaign_id: 'c1' })).text).toBe('id: phare\n')
    const r = JSON.parse((await call(c, 'import_campaign_yaml', { campaign_id: 'c1', yaml: 'id: phare\n' })).text)
    expect(api.sent[1]!.body).toEqual({ yaml: 'id: phare\n' })
    expect(r.title).toBe('Le Phare de Kerbrume')
    expect(r.issues.errors).toBe(1)
  })

  it('creates a new campaign from a YAML file', async () => {
    const api = mockFetch({
      'POST /campaigns/import': () => ({ status: 201, body: { data: DETAIL } }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'create_campaign_from_yaml', { yaml: 'id: phare\n' })
    expect(r.isError).toBe(false)
    expect(api.sent[0]!.body).toEqual({ yaml: 'id: phare\n' })
    const created = JSON.parse(r.text)
    expect(created.title).toBe('Le Phare de Kerbrume')
    expect(created.issues.errors).toBe(1)
  })

  it('says when the token is revoked, the campaign unknown or the server down', async () => {
    const api = mockFetch({
      'GET /campaigns': () => ({ status: 401, body: { error: { code: 'INVALID_TOKEN', message: 'not signed in' } } }),
      'GET /campaigns/nope': () => ({ status: 404, body: { error: { code: 'NOT_FOUND', message: 'not found' } } }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const revoked = await call(c, 'list_campaigns')
    expect(revoked.isError).toBe(true)
    expect(revoked.text).toContain('révoqué ou inconnu')
    expect(revoked.text).toContain('Accès pour Claude')
    const unknown = await call(c, 'get_campaign', { campaign_id: 'nope' })
    expect(unknown.text).toContain('Campagne introuvable')

    const down = (async () => {
      throw new TypeError('fetch failed')
    }) as unknown as typeof fetch
    await client?.close()
    const c2 = await connect(ENV, down)
    const r = await call(c2, 'list_campaigns')
    expect(r.isError).toBe(true)
    expect(r.text).toContain('Promptus est injoignable à http://promptus.test (fetch failed)')
  })

  it('summarizes the atlas: the tree, what stands on each map, what has no pin', async () => {
    const api = mockFetch({
      'GET /campaigns/c1/atlas': () => ({
        status: 200,
        body: {
          data: {
            revision: 4,
            roots: ['cotes'],
            claude: [{}],
            claudeHand: false,
            maps: [
              {
                map: { id: 'cotes', name: 'Les côtes', scale: 'world', grid: { rows: ['~~~', '###'] } },
                origin: 'world',
                parent: null,
                children: ['rues'],
                validated: true,
                pins: [{ location: 'lieu_port', name: 'Le port', at: [2, 1], scenes: [{ id: 'sc_port', title: 'Le port' }] }],
                cast: { npcs: [{ npc: 'pnj_gwen', at: [1, 1] }], clues: [], fights: [] },
                proposed: [],
              },
            ],
            places: [{ id: 'lieu_port', name: 'Le port', parent: null, anchor: { map: 'cotes', at: [2, 1] } }],
            unplaced: [{ id: 'sc_reve', title: 'Le rêve' }],
            issues: [],
            frieze: {
              acts: [
                {
                  act: 'acte_1',
                  title: 'L’Offre',
                  evening: 1,
                  slots: [
                    { stops: [{ place: 'lieu_taverne', name: 'La taverne', scenes: [{ id: 'sc_taverne', title: 'La taverne' }] }] },
                    {
                      stops: [
                        { map: 'rues', name: 'Les rues', scenes: [{ id: 'sc_marche', title: 'Le marché' }] },
                        { map: 'quai', name: 'Le quai', scenes: [{ id: 'sc_chantier', title: 'Le chantier' }] },
                      ],
                    },
                  ],
                },
                { act: 'acte_2', title: 'Le large', evening: 2, slots: [] },
              ],
              outside: ['cotes'],
            },
          },
        },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const a = JSON.parse((await call(c, 'get_atlas', { campaign_id: 'c1' })).text)
    expect(a.lots_en_attente).toBe(1)
    expect(a.cartes[0]).toMatchObject({ id: 'cotes', taille: '3 × 2', contient: ['rues'] })
    expect(a.cartes[0].épingles[0]).toEqual({ lieu: 'lieu_port', nom: 'Le port', case: '[2, 1]', scènes: ['sc_port'] })
    expect(a.cartes[0].pnj).toEqual([{ pnj: 'pnj_gwen', case: '[1, 1]' }])
    expect(a.lieux[0].épingle).toBe('cotes [2, 1]')
    expect(a.scènes_sans_épingle).toEqual(['sc_reve (Le rêve)'])
    // The story's order, act by act: a map to create names its place, a branch is one slot of several maps.
    expect(a.frise[0]).toEqual({
      soir: 1,
      acte: 'acte_1',
      titre: 'L’Offre',
      cases: [
        [{ carte_à_créer_pour: 'lieu_taverne', nom: 'La taverne', scènes: ['sc_taverne'] }],
        [
          { carte: 'rues', nom: 'Les rues', scènes: ['sc_marche'] },
          { carte: 'quai', nom: 'Le quai', scènes: ['sc_chantier'] },
        ],
      ],
    })
    expect(a.frise[1].cases).toEqual([])
    expect(a.hors_de_l_histoire).toEqual(['cotes'])
  })

  it('draws a map as text with its legend, a zone on demand', async () => {
    const api = mockFetch({
      'GET /campaigns/c1/maps/rues/sketch': () => ({
        status: 200,
        body: {
          data: {
            map: 'rues',
            name: 'Les rues',
            scale: 'place',
            width: 40,
            height: 30,
            zone: { x: 0, y: 0, w: 5, h: 2 },
            cut: true,
            rows: ['    0', '    01234', '  0 #####', '  1 #aA~#'],
            terrain: [
              { glyph: '#', terrain: 'pierre', wall: true, void: false, water: 'none', difficult: false, elevation: 0 },
              { glyph: '~', terrain: 'eau', wall: false, void: false, water: 'deep', difficult: false, elevation: 0 },
            ],
            legend: [
              { symbol: 'A', kind: 'npc', id: 'pnj_gwen', name: 'Gwen', cells: [[2, 1]], note: 'sc_taverne' },
              { symbol: 'a', kind: 'prop', id: 'comptoir', name: 'Le comptoir', cells: [[1, 1]], layer: 'gm', note: 'comptoir' },
            ],
          },
        },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'get_map_text', { campaign_id: 'c1', map_id: 'rues', x: 0, w: 5 })
    expect(r.isError).toBe(false)
    expect(api.sent[0]!.key).toBe('GET /campaigns/c1/maps/rues/sketch')
    expect(r.text).toContain('  1 #aA~#')
    expect(r.text).toContain('  ~  eau (eau profonde)')
    expect(r.text).toContain('  A  PNJ pnj_gwen (Gwen) · [2, 1] · scène sc_taverne')
    expect(r.text).toContain('  a  Décor comptoir (Le comptoir) · [1, 1] · comptoir · MJ seul')
    expect(r.text).toContain('demande une autre zone')
  })

  it('places by name and says where each thing landed, proposed to the GM', async () => {
    const placements = [
      { what: 'npc', id: 'pnj_gwen', near: 'comptoir' },
      { what: 'clue', id: 'cl_filet', along: { from: 'pj-1', to: 'la rue' } },
    ]
    const api = mockFetch({
      'POST /campaigns/c1/maps/rues/place': () => ({
        status: 202,
        body: {
          data: {
            status: 'proposed',
            why: 'proposals',
            proposal: { id: 'p1', prompt: 'Peupler la taverne', status: 'pending', items: [{}, {}], changes: [], createdAt: '' },
            issues: [],
            placed: [
              { what: 'npc', id: 'pnj_gwen', name: 'Gwen', at: [3, 2], near: 'comptoir (Le comptoir)', steps: 1, terrain: 'plancher' },
              { what: 'clue', id: 'cl_filet', name: 'Un filet', at: [6, 4], near: 'pj-1 → la rue', steps: 0, terrain: 'pavés' },
            ],
          },
        },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'place_on_map', { campaign_id: 'c1', map_id: 'rues', placements, title: 'Peupler la taverne' })
    expect(r.isError).toBe(false)
    expect(api.sent[0]!.body).toEqual({ placements, title: 'Peupler la taverne' })
    const out = JSON.parse(r.text)
    expect(out.status).toBe('proposé')
    expect(out.placed).toEqual([
      'PNJ Gwen (pnj_gwen) → [3, 2], plancher, à 1 pas de « comptoir (Le comptoir) »',
      'Indice Un filet (cl_filet) → [6, 4], pavés, sur « pj-1 → la rue »',
    ])
    expect(out.proposal).toEqual({ id: 'p1', title: 'Peupler la taverne', items: 2 })
  })

  it("explains a reference the map does not know, with the server's candidates", async () => {
    const api = mockFetch({
      'POST /campaigns/c1/maps/rues/place': () => ({
        status: 400,
        body: { error: { code: 'MAP_REF_UNKNOWN', message: 'nothing named "cimetière" on this map; known: comptoir (Le comptoir)' } },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'place_on_map', {
      campaign_id: 'c1',
      map_id: 'rues',
      placements: [{ what: 'npc', id: 'pnj_oswin', near: 'cimetière' }],
    })
    expect(r.isError).toBe(true)
    expect(r.text).toContain('get_map_text')
    expect(r.text).toContain('known: comptoir (Le comptoir)')
  })

  it('asks the engine the way and its cost', async () => {
    const api = mockFetch({
      'POST /campaigns/c1/maps/rues/ask': () => ({
        status: 200,
        body: {
          data: {
            question: 'path',
            from: { name: 'pj-1', cells: [[0, 4]] },
            to: { name: 'la rue', cells: [[8, 5]] },
            path: [[0, 4], [1, 4], [2, 5]],
            cost: 2,
            meters: 10,
          },
        },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = JSON.parse((await call(c, 'ask_map', { campaign_id: 'c1', map_id: 'rues', question: 'path', from: 'pj-1', to: 'la rue' })).text)
    expect(api.sent[0]!.body).toEqual({ question: 'path', from: 'pj-1', to: 'la rue' })
    expect(r).toEqual({ de: 'pj-1', à: 'la rue', coût: '2 case(s) de déplacement', mètres: 10, chemin: '[0, 4] [1, 4] [2, 5]' })
    const missing = await call(c, 'ask_map', { campaign_id: 'c1', map_id: 'rues', question: 'sight', from: 'pj-1' })
    expect(missing.isError).toBe(true)
    expect(api.sent.length).toBe(1)
  })

  it('sends map operations on the revision read, and replays once on the fresh one', async () => {
    let tries = 0
    const api = mockFetch({
      'GET /campaigns/c1/maps/crypte': () => ({ status: 200, body: { data: { revision: 3 } } }),
      'POST /campaigns/c1/maps/crypte/ops': () => {
        tries += 1
        return tries === 1
          ? { status: 409, body: { error: { code: 'MAP_STALE', message: '', fresh: { revision: 5 } } } }
          : {
              status: 202,
              body: { data: { status: 'proposed', why: 'proposals', proposal: { id: 'p2', prompt: '', items: [{}] }, issues: [] } },
            }
      },
    })
    const c = await connect(ENV, api.fetchImpl)
    const ops = [{ op: 'door', at: [3, 0] }]
    const r = await call(c, 'edit_map', { campaign_id: 'c1', map_id: 'crypte', ops })
    expect(r.isError).toBe(false)
    expect(api.sent.map((x) => (x.body as { revision?: number } | undefined)?.revision)).toEqual([undefined, 3, 5])
    expect(JSON.parse(r.text).status).toBe('proposé')
  })

  it('says what to set when the configuration is missing', async () => {
    const c = await connect({ PROMPTUS_URL: 'http://promptus.test' }, mockFetch({}).fetchImpl)
    const r = await call(c, 'list_campaigns')
    expect(r.isError).toBe(true)
    expect(r.text).toContain('PROMPTUS_URL et PROMPTUS_TOKEN doivent être définis')
  })

  describe('preparing with Claude (campaign/prepare-with-claude)', () => {
    const PROPOSED = (items: number) => ({
      status: 202,
      body: {
        data: {
          status: 'proposed',
          why: 'proposals',
          proposal: { id: 'p1', prompt: 'lot', status: 'pending', items: Array.from({ length: items }, (_, i) => ({ key: `e${i}`, status: 'pending', stale: false })), changes: [], createdAt: '' },
          issues: [],
        },
      },
    })

    it('says « Jouable » once nothing is missing', async () => {
      const api = mockFetch({
        'GET /campaigns/c1': () => ({ status: 200, body: { data: { ...DETAIL, issues: [] } } }),
        'GET /campaigns/c1/readiness': () => ({ status: 200, body: { data: [{ act: 'acte_1', title: 'La brume', ready: true }] } }),
      })
      const c = await connect(ENV, api.fetchImpl)
      const r = JSON.parse((await call(c, 'get_readiness', { campaign_id: 'c1' })).text)
      expect(r.playable.verdict).toBe('Jouable')
    })

    it('starts a campaign from one scene, the rules left for the first fight', async () => {
      const api = mockFetch({
        'GET /rule-systems': () => ({ status: 200, body: { data: [{ id: 'corsaires', version: 1, name: 'Corsaires' }] } }),
        'POST /campaigns/import': () => ({ status: 201, body: { data: { id: 'c9', story: {}, issues: [] } } }),
      })
      const c = await connect(ENV, api.fetchImpl)
      const r = await call(c, 'start_campaign', {
        title: 'La Mâchoire',
        world: 'Bretagne, 1718',
        tone: 'Sel et poudre',
        scene: { title: 'La bagarre du quai', summary: 'Six contre six.', ending: true },
      })
      expect(r.isError).toBe(false)
      const out = JSON.parse(r.text)
      expect(out).toMatchObject({ id: 'c9', scene: { id: 'sc_la_bagarre_du_quai', ending: true } })
      expect(out.rules.provisional).toBeDefined()
      // A finale is not the opening: the scene kept upstream will be.
      const story = JSON.parse((api.sent.at(-1)!.body as { yaml: string }).yaml)
      expect(story).toMatchObject({
        id: 'la_machoire',
        rules: { id: 'corsaires', version: 1 },
        bible: { tone: 'Sel et poudre' },
        nodes: [{ id: 'sc_la_bagarre_du_quai', act: 'acte_1', title: 'La bagarre du quai' }],
      })
      expect(story.bible.start_node).toBeUndefined()
    })

    it('proposes a tension with its clock, never applies it', async () => {
      const api = mockFetch({
        'GET /campaigns/c1': () => ({ status: 200, body: { data: DETAIL } }),
        'POST /campaigns/c1/story/edits': () => PROPOSED(1),
      })
      const c = await connect(ENV, api.fetchImpl)
      const steps = ['Rumeurs', 'Menaces', 'Incendie', 'Arrestations', 'Blocus', 'Le port tombe'].map((label) => ({ label }))
      const r = JSON.parse((await call(c, 'propose_tension', { campaign_id: 'c1', name: 'La Compagnie', goal: 'Racheter le port', steps })).text)
      expect(r).toMatchObject({ status: 'proposé', front: 'front_la_compagnie', clock: 6 })
      const body = api.sent.at(-1)!.body as { edits: { value: { steps: unknown[] } }[]; title: string }
      expect(body.edits[0]!.value.steps).toHaveLength(6)
      expect(body.title).toBe('Tension : La Compagnie')
      expect((await call(c, 'propose_tension', { campaign_id: 'c1', name: 'x', goal: 'y', steps: steps.slice(0, 3) })).isError).toBe(true)
    })

    it('proposes a scene backwards from the end, with the clue found there, as the new opening', async () => {
      const api = mockFetch({
        'GET /campaigns/c1': () => ({ status: 200, body: { data: DETAIL } }),
        'POST /campaigns/c1/story/edits': () => PROPOSED(3),
      })
      const c = await connect(ENV, api.fetchImpl)
      const r = JSON.parse(
        (await call(c, 'propose_scene', {
          campaign_id: 'c1',
          scene: { title: 'Le naufrage', summary: 'Une coque échouée.' },
          before: 'sc_auberge',
          opening: true,
          clues: ['cl_filet'],
        })).text,
      )
      expect(r).toMatchObject({ status: 'proposé', scene: 'sc_le_naufrage', direction: 'à rebours' })
      const { edits } = api.sent.at(-1)!.body as { edits: unknown[] }
      expect(edits).toEqual([
        { op: 'add', kind: 'node', value: { id: 'sc_le_naufrage', act: 'acte_1', title: 'Le naufrage', summary: 'Une coque échouée.', exits: [{ to: 'sc_auberge', label: "L'auberge" }] } },
        { op: 'set', target: 'bible', field: 'start_node', value: 'sc_le_naufrage' },
        { op: 'set', target: 'cl_filet', field: 'node', value: 'sc_le_naufrage' },
      ])
    })

    it('adds an exit to the scene before when going forward, keeping its others', async () => {
      const api = mockFetch({
        'GET /campaigns/c1': () => ({ status: 200, body: { data: DETAIL } }),
        'POST /campaigns/c1/story/edits': () => PROPOSED(2),
      })
      const c = await connect(ENV, api.fetchImpl)
      await call(c, 'propose_scene', { campaign_id: 'c1', scene: { title: 'Quai', summary: 'Encore.' }, after: 'sc_auberge', exit_label: 'Suivre la lanterne' })
      const { edits } = api.sent.at(-1)!.body as { edits: unknown[] }
      // `sc_quai` exists: the new id is numbered.
      expect(edits[1]).toEqual({
        op: 'set',
        target: 'sc_auberge',
        field: 'exits',
        value: [{ to: 'sc_quai', label: 'Le quai' }, { to: 'sc_quai_2', label: 'Suivre la lanterne' }],
      })
      const unknown = await call(c, 'propose_scene', { campaign_id: 'c1', scene: { title: 'x', summary: '' }, after: 'sc_nulle_part' })
      expect(unknown.isError).toBe(true)
      expect(unknown.text).toContain('sc_auberge')
      const unlinked = await call(c, 'propose_scene', { campaign_id: 'c1', scene: { title: 'x', summary: '' } })
      expect(unlinked.isError).toBe(true)
    })

    it('serves the preparation skill as a prompt', async () => {
      const c = await connect(ENV, mockFetch({}).fetchImpl)
      const { prompts } = await c.listPrompts()
      expect(prompts.map((p) => p.name)).toContain('preparer_soiree')
      const got = await c.getPrompt({ name: 'preparer_soiree', arguments: { depart: 'des corsaires' } })
      const body = (got.messages[0]!.content as { text: string }).text
      expect(body).toContain('# Préparer ma prochaine soirée')
      expect(body).not.toContain('name: preparer-soiree')
      expect(body).toContain('Départ du MJ : des corsaires')
    })
  })

  describe('PixelLab (campaign/import-pixellab-assets)', () => {
    it('says what is missing, how to draw it and what it costs, and never asks for the rest', async () => {
      const api = mockFetch({
        'GET /campaigns/c1/elements': () => ({
          status: 200,
          body: {
            data: {
              tileset: 'port-1718',
              palette: ['#5d5850'],
              generations: 7,
              needs: [
                { kind: 'element', subject: 'port-1718/material/pavés', name: 'Sol · Pavés', form: 'texture', status: 'missing', generations: 1 },
                { kind: 'element', subject: 'port-1718/wall', name: 'Murs', form: 'texture', status: 'approved', generations: 0 },
                { kind: 'character', subject: 'pnj_morel', name: 'Morel', form: 'character', status: 'missing', generations: 6 },
              ],
            },
          },
        }),
      })
      const c = await connect(ENV, api.fetchImpl)
      const r = JSON.parse((await call(c, 'get_theme_needs', { campaign_id: 'c1', map_id: 'quai' })).text)
      expect(r.missing.map((m: { subject: string }) => m.subject)).toEqual(['port-1718/material/pavés', 'pnj_morel'])
      expect(r.missing[0].how).toContain('view: side')
      expect(r.already).toEqual([{ subject: 'port-1718/wall', name: 'Murs', status: 'gardé par le MJ (ne pas regénérer)' }])
      expect(r.next).toContain('7 génération(s)')
      expect(api.sent[0]!.key).toBe('GET /campaigns/c1/elements')
    })

    it('drops an image in, pending, and explains a slot already filled', async () => {
      let first = true
      const api = mockFetch({
        'POST /campaigns/c1/media/import': () => {
          if (first) {
            first = false
            return { status: 201, body: { data: { id: 'a1', status: 'pending', meta: { name: 'Sol · Pavés' } } } }
          }
          return { status: 409, body: { error: { code: 'ELEMENT_EXISTS', message: 'x' } } }
        },
      })
      const c = await connect(ENV, api.fetchImpl)
      const args = { campaign_id: 'c1', kind: 'element', subject: 'port-1718/material/pavés', url: 'https://api.pixellab.ai/x.png', generations: 1 }
      const ok = JSON.parse((await call(c, 'import_pixellab_asset', args)).text)
      expect(ok).toMatchObject({ status: 'en attente du MJ', id: 'a1' })
      expect(api.sent[0]!.body).toEqual({ kind: 'element', subject: 'port-1718/material/pavés', url: 'https://api.pixellab.ai/x.png', generations: 1 })
      const again = await call(c, 'import_pixellab_asset', args)
      expect(again.isError).toBe(true)
      expect(again.text).toContain('ne regénère pas')
    })
  })
})
