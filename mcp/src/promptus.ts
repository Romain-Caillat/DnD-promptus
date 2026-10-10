/**
 * The Promptus API as the MCP server reaches it: one GM personal access
 * token, sent as `Authorization: Bearer`, on the preparation routes the
 * server opens to tokens (`back/src/auth/api_tokens.rs`).
 */

export interface Config {
  /** The origin Promptus is served at, without `/api`. */
  url: string
  token: string
}

/** A refused or failed call, with a message written for the GM (French). */
export class PromptusError extends Error {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string, message: string) {
    super(message)
    this.name = 'PromptusError'
    this.status = status
    this.code = code
  }
}

const HELP = "Dans Promptus : espace MJ, panneau « Accès pour Claude »."

/** The configuration from the environment, or why it is unusable. */
export function readConfig(env: Record<string, string | undefined>): Config | PromptusError {
  const url = env.PROMPTUS_URL?.trim().replace(/\/+$/, '').replace(/\/api$/, '')
  const token = env.PROMPTUS_TOKEN?.trim()
  if (!url || !token) {
    return new PromptusError(
      0,
      'NOT_CONFIGURED',
      `PROMPTUS_URL et PROMPTUS_TOKEN doivent être définis dans la configuration du serveur MCP ` +
        `(claude_desktop_config.json, ou « claude mcp add -e »). ${HELP}`,
    )
  }
  try {
    const parsed = new URL(url)
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') throw new Error('protocol')
  } catch {
    return new PromptusError(0, 'NOT_CONFIGURED', `PROMPTUS_URL n'est pas une adresse web valide : « ${url} ».`)
  }
  return { url, token }
}

/** What the GM reads for a server code (the server's own detail is appended). */
function explain(status: number, code: string, detail: string | null): string {
  if (code === 'INVALID_TOKEN' || (status === 401 && code === 'UNAUTHENTICATED')) {
    return `Promptus a refusé le jeton (révoqué ou inconnu). Crée un nouveau jeton. ${HELP}`
  }
  if (code === 'TOKEN_NOT_ALLOWED') {
    return "Ce jeton n'ouvre pas cette action : il sert à la préparation des campagnes seulement."
  }
  if (status === 404) {
    return "Campagne introuvable : l'identifiant est faux, ou la campagne appartient à un autre MJ. " +
      'Utilise list_campaigns pour retrouver les identifiants.'
  }
  const why = detail ? ` Détail du serveur : ${detail}` : ''
  if (code.startsWith('EDIT_')) {
    return `Modification refusée (${code}) : rien n'a été appliqué, c'est tout ou rien.${why}`
  }
  if (code === 'TOO_MANY_EDITS') return 'Trop de modifications en une fois (200 au plus) : découpe-les.'
  if (code === 'INVALID_CAMPAIGN_YAML') return `Ce YAML n'est pas une campagne lisible : rien n'a été importé.${why}`
  if (code === 'INVALID_BODY') return `Requête mal formée (INVALID_BODY).${why}`
  return `Promptus a répondu ${status} (${code}).${why}`
}

export class Promptus {
  private readonly config: Config
  private readonly fetchImpl: typeof fetch

  constructor(config: Config, fetchImpl: typeof fetch = fetch) {
    this.config = config
    this.fetchImpl = fetchImpl
  }

  /** Call the API and return the `data` of its envelope; throws `PromptusError`. */
  async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = `${this.config.url}/api${path}`
    let response: Response
    try {
      response = await this.fetchImpl(url, {
        method,
        headers: {
          Authorization: `Bearer ${this.config.token}`,
          Accept: 'application/json',
          ...(body === undefined ? {} : { 'Content-Type': 'application/json' }),
        },
        body: body === undefined ? undefined : JSON.stringify(body),
      })
    } catch (err) {
      const why = err instanceof Error ? err.message : String(err)
      throw new PromptusError(
        0,
        'UNREACHABLE',
        `Promptus est injoignable à ${this.config.url} (${why}). Le serveur tourne-t-il, et PROMPTUS_URL est-elle la bonne ?`,
      )
    }
    const json: unknown = await response.json().catch(() => null)
    if (!response.ok) {
      const error = (json as { error?: { code?: unknown; message?: unknown } } | null)?.error
      const code = typeof error?.code === 'string' ? error.code : 'UNEXPECTED'
      const detail = typeof error?.message === 'string' ? error.message : null
      throw new PromptusError(response.status, code, explain(response.status, code, detail))
    }
    if (typeof json !== 'object' || json === null || !('data' in json)) {
      throw new PromptusError(
        response.status,
        'UNEXPECTED',
        `Réponse inattendue de ${url} : est-ce bien un serveur Promptus ?`,
      )
    }
    return (json as { data: T }).data
  }
}
