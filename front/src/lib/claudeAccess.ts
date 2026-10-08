import { apiRequest } from './api'

/** A personal access token, as its GM sees it in the list. Never the secret. */
export interface ApiToken {
  id: string
  name: string
  createdAt: string
  lastUsedAt: string | null
}

export function listTokens(): Promise<ApiToken[]> {
  return apiRequest<ApiToken[]>('GET', '/gm-tokens')
}

/** Mint a named token: the only answer that carries its secret. */
export function createToken(name: string): Promise<ApiToken & { secret: string }> {
  return apiRequest<ApiToken & { secret: string }>('POST', '/gm-tokens', { name })
}

export function revokeToken(id: string): Promise<void> {
  return apiRequest<void>('DELETE', `/gm-tokens/${encodeURIComponent(id)}`)
}

/** Where the MCP server's entry point sits in a checkout of Promptus. */
const MCP_ENTRY = '/CHEMIN/VERS/DnD-promptus/mcp/src/index.ts'
/** Stands for the secret once it is no longer shown. */
export const TOKEN_PLACEHOLDER = 'COLLE_TON_JETON_ICI'

/** The `mcpServers` entry of Claude Desktop's `claude_desktop_config.json`. */
export function desktopConfig(origin: string, token: string): string {
  return JSON.stringify(
    {
      mcpServers: {
        promptus: {
          command: 'bun',
          args: ['run', MCP_ENTRY],
          env: { PROMPTUS_URL: origin, PROMPTUS_TOKEN: token },
        },
      },
    },
    null,
    2,
  )
}

/** The same server declared to Claude Code, for every project of the user. */
export function claudeCodeCommand(origin: string, token: string): string {
  return `claude mcp add promptus -s user -e PROMPTUS_URL=${origin} -e PROMPTUS_TOKEN=${token} -- bun run ${MCP_ENTRY}`
}
