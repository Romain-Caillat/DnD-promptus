import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import { Promptus, readConfig } from './promptus'
import { registerMapTools } from './maps'
import { registerTools } from './tools'

/**
 * The MCP server over `env` (PROMPTUS_URL, PROMPTUS_TOKEN). A missing
 * or bad configuration does not stop it: every tool then answers why,
 * which Claude shows the GM in the conversation.
 */
export function createServer(env: Record<string, string | undefined>, fetchImpl: typeof fetch = fetch) {
  const server = new McpServer({ name: 'promptus', version: '0.1.0' })
  const config = readConfig(env)
  const client = config instanceof Error ? null : new Promptus(config, fetchImpl)
  const promptus = () => {
    if (client === null) throw config
    return client
  }
  registerTools(server, promptus)
  registerMapTools(server, promptus)
  return server
}
