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
  it('offers the seven preparation tools, described in French', async () => {
    const c = await connect(ENV, mockFetch({}).fetchImpl)
    const { tools } = await c.listTools()
    expect(tools.map((t) => t.name).sort()).toEqual([
      'apply_story_edits',
      'create_campaign_from_yaml',
      'export_campaign_yaml',
      'get_campaign',
      'get_readiness',
      'get_scene',
      'import_campaign_yaml',
      'list_campaigns',
    ])
    const edits = tools.find((t) => t.name === 'apply_story_edits')!
    expect(edits.description).toContain('tout ou rien')
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
  })

  it('passes the edits to the server as they are, and reports what changed', async () => {
    const edit = { op: 'add', kind: 'clue', value: { id: 'cl_quai', revelation: 'rv_gardien', node: 'sc_auberge', text: 'Un indice vers le quai.' } }
    const api = mockFetch({
      'POST /campaigns/c1/story/edits': () => ({
        status: 200,
        body: { data: { campaign: { ...DETAIL, issues: [] }, changes: [{ kind: 'added', id: 'cl_quai' }] } },
      }),
    })
    const c = await connect(ENV, api.fetchImpl)
    const r = await call(c, 'apply_story_edits', { campaign_id: 'c1', edits: [edit] })
    expect(r.isError).toBe(false)
    expect(api.sent[0]!.body).toEqual({ edits: [edit] })
    expect(JSON.parse(r.text)).toEqual({
      changes: [{ kind: 'added', id: 'cl_quai' }],
      issues: { errors: 0, warnings: 0, list: [] },
    })
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

  it('says what to set when the configuration is missing', async () => {
    const c = await connect({ PROMPTUS_URL: 'http://promptus.test' }, mockFetch({}).fetchImpl)
    const r = await call(c, 'list_campaigns')
    expect(r.isError).toBe(true)
    expect(r.text).toContain('PROMPTUS_URL et PROMPTUS_TOKEN doivent être définis')
  })
})
