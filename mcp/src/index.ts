#!/usr/bin/env bun
/**
 * Entry point launched by Claude Desktop or Claude Code on the GM's
 * machine: an MCP server on stdio. stdout carries the protocol, so
 * anything for humans goes to stderr.
 */
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'
import { readConfig } from './promptus'
import { createServer } from './server'

const config = readConfig(process.env)
if (config instanceof Error) console.error(`[promptus-mcp] ${config.message}`)
else console.error(`[promptus-mcp] Promptus at ${config.url}`)

await createServer(process.env).connect(new StdioServerTransport())
