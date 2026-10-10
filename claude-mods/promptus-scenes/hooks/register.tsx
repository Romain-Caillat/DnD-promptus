import { atom, read, update } from 'claude-code'
import type { EngineInterface, Register } from 'claude-code'

import type { Act, Change, Scan, Scene } from '../types'
import { SCAN_RB } from './scan-rb'

const PANE = 'promptus-scenes'
const TITLE = 'Promptus · scènes'
const ALL = '*'
const DEFAULT_ROOT = '~/02_perso/DnD-promptus'

const scanAtom = atom({ plugin: 'promptus-scenes', key: 'scan' } as const, null)
const actAtom = atom({ plugin: 'promptus-scenes', key: 'act' } as const, ALL)
const sceneAtom = atom({ plugin: 'promptus-scenes', key: 'scene' } as const, '')
const changesAtom = atom({ plugin: 'promptus-scenes', key: 'changes' } as const, {})

const MOODS: Record<string, string> = {
  calm: 'calme',
  exploration: 'exploration',
  tension: 'tension',
  mystery: 'mystère',
  combat: 'combat',
  epic: 'épique',
}

const allScenes = (scan: Scan): Scene[] => scan.acts.flatMap(act => act.scenes)

let refreshing: Promise<void> | null = null

async function refresh($: EngineInterface, campaign?: string): Promise<void> {
  if (refreshing) return refreshing
  refreshing = (async () => {
    const previous = (await read($, scanAtom)) as Scan | null
    const storedRoot = ((await $.store.get('root')) as string | undefined) ?? ''
    const key = campaign ?? previous?.key ?? (((await $.store.get('campaign')) as string) || '')
    const roots = [await $.session.cwd(), storedRoot, DEFAULT_ROOT].filter(Boolean)
    const ran = await $.process.run(['ruby', '-e', SCAN_RB], {
      stdin: JSON.stringify({ roots, campaign: key }),
      env: { LC_ALL: 'en_US.UTF-8', LANG: 'en_US.UTF-8' },
      timeoutMs: 15000,
    })
    let next: Scan
    try {
      next = JSON.parse(ran.stdout) as Scan
    } catch {
      const why = ran.stderr.split('\n')[0] || `ruby a quitté avec le code ${ran.exitCode}`
      next = { root: '', campaigns: [], key: '', title: '', path: '', mtimeMs: 0, acts: [], error: why }
    }
    next.campaigns ??= []
    next.acts ??= []

    if (next.root) await $.store.set('root', next.root)
    if (next.key) await $.store.set('campaign', next.key)

    if (!previous || previous.key !== next.key) {
      await update($, changesAtom, () => ({}))
      await update($, actAtom, () => ALL)
      await update($, sceneAtom, () => allScenes(next)[0]?.id ?? '')
    } else {
      const before = new Map(allScenes(previous).map(s => [s.id, s.hash]))
      const fresh: Record<string, Change> = {}
      for (const s of allScenes(next)) {
        if (!before.has(s.id)) fresh[s.id] = 'new'
        else if (before.get(s.id) !== s.hash) fresh[s.id] = 'changed'
      }
      const ids = Object.keys(fresh)
      if (ids.length > 0) {
        await update($, changesAtom, old => ({ ...old, ...fresh }))
        await update($, sceneAtom, () => ids[ids.length - 1])
        const titles = allScenes(next)
          .filter(s => fresh[s.id])
          .map(s => `« ${s.title} »`)
        const verb = ids.every(id => fresh[id] === 'new') ? 'ajoutée' : 'mise à jour'
        $.ui.toast(`Promptus : ${titles.slice(0, 3).join(', ')} ${verb}${ids.length > 1 ? 's' : ''}`)
      }
    }
    await update($, scanAtom, () => next)
  })().finally(() => {
    refreshing = null
  })
  return refreshing
}

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    await $.command.register({
      name: 'promptus',
      description: 'Promptus : les scènes de la campagne et leurs assets (argument : campagne ou dossier du projet)',
    })
    await refresh($)
    void $.ui.open({ id: PANE, title: TITLE })

    // A file edited outside a tool call (the GM's editor, a script) shows too.
    $.clock.every(4000, () => {
      void (async () => {
        const scan = (await read($, scanAtom)) as Scan | null
        if (!scan?.path) return
        const stat = await $.fs.stat(scan.path).catch(() => null)
        if (stat && Math.floor(stat.mtimeMs) > scan.mtimeMs + 1) await refresh($)
      })()
    })

    return next(e)
  })

  on('command.run', { command: 'promptus' }, async ($, e) => {
    const arg = e.args.trim()
    if (arg.includes('/') || arg.startsWith('~')) {
      await $.store.set('root', arg)
      await update($, scanAtom, () => null)
      await refresh($, '')
    } else {
      await refresh($, arg || undefined)
    }
    const opened = await $.ui.open({ id: PANE, title: TITLE })
    const scan = (await read($, scanAtom)) as Scan | null
    if (scan?.error) return { text: `Promptus : ${scan.error}` }
    const where = opened.isPlaced ? '' : ' (élargis la fenêtre pour voir le panneau)'
    return { text: `Panneau Promptus ouvert sur « ${scan?.title ?? '?'} »${where}.` }
  })

  on('tool.call', async ($, e, next) => {
    const ran = await next(e)
    const isWrite = ['Write', 'Edit', 'MultiEdit'].includes(e.tool)
    if (isWrite && /content\/[^"]*\.ya?ml/.test(JSON.stringify(e))) void refresh($)
    return ran
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const { Box, Text, Button, Select } = $.ui.resolve(e) as any
    const scan = (await read($, scanAtom)) as Scan | null
    const actId = (await read($, actAtom)) as string
    const sceneId = (await read($, sceneAtom)) as string
    const changes = (await read($, changesAtom)) as Record<string, Change>
    const rows = e.viewport?.rows ?? 40

    if (!scan) return <Text dimColor>Lecture de la campagne…</Text>
    if (scan.error && scan.acts.length === 0) {
      return (
        <Box flexDirection="column">
          <Text color="red">{scan.error}</Text>
          <Button key="retry" label="Relire" onPress={() => void refresh($)} />
        </Box>
      )
    }

    const acts: Act[] = actId === ALL ? scan.acts : scan.acts.filter(a => a.id === actId)
    const shown = acts.flatMap(a => a.scenes)
    const scene = shown.find(s => s.id === sceneId) ?? shown[0]

    const header = (
      <Box flexDirection="column" key="header">
        <Box gap={1}>
          <Text bold>{scan.title}</Text>
          <Text dimColor>
            {allScenes(scan).length} scènes · {scan.acts.length} actes
          </Text>
          <Button key="reload" plain label="↻" onPress={() => void refresh($)} />
        </Box>
        <Box gap={2}>
          <Select
            key="campaign"
            label="Campagne"
            value={scan.key}
            options={scan.campaigns.map(c => ({
              value: c.key,
              label: c.isExample ? `${c.title} (exemple)` : c.title,
            }))}
            onSelect={(value: string) => void refresh($, value)}
          />
          <Select
            key="act"
            label="Acte"
            value={actId}
            options={[
              { value: ALL, label: 'Tous les actes' },
              ...scan.acts.map(a => ({ value: a.id, label: `${a.title} (${a.scenes.length})` })),
            ]}
            onSelect={(value: string) => {
              void update($, actAtom, () => value)
              const first = scan.acts.find(a => a.id === value)?.scenes[0] ?? allScenes(scan)[0]
              void update($, sceneAtom, () => first?.id ?? '')
            }}
          />
        </Box>
      </Box>
    )

    const listRoom = Math.max(3, Math.floor(rows * 0.4))
    const list = (
      <Box flexDirection="column" key="list" marginTop={1}>
        {acts.map(act => (
          <Box flexDirection="column" key={`act-${act.id}`}>
            <Text bold color="cyan">
              {act.title}
              {act.scenes.length === 0 ? '  — aucune scène' : ''}
            </Text>
            {act.scenes.slice(0, listRoom).map(s => (
              <Box key={`row-${s.id}`} gap={1}>
                <Text color={s.id === scene?.id ? 'yellow' : undefined}>{s.id === scene?.id ? '›' : ' '}</Text>
                <Button
                  key={`pick-${s.id}`}
                  plain
                  label={s.title}
                  onPress={() => void update($, sceneAtom, () => s.id)}
                />
                <Text wrap="truncate-end" dimColor>
                  {badges(s)}
                </Text>
                {changes[s.id] && <Text color="green">{changes[s.id] === 'new' ? 'nouvelle' : 'modifiée'}</Text>}
              </Box>
            ))}
          </Box>
        ))}
      </Box>
    )

    const detail = scene && (
      <Box flexDirection="column" key="detail" marginTop={1} borderStyle="round" paddingX={1}>
        <Box gap={1}>
          <Text bold>{scene.title}</Text>
          {scene.optional && <Text dimColor>(facultative)</Text>}
          {scene.invented && <Text color="magenta">inventé — à valider</Text>}
        </Box>
        {scene.summary ? <Text wrap="wrap">{scene.summary}</Text> : <Text color="red">Pas de résumé.</Text>}
        <Text wrap="truncate-end">
          <Text dimColor>Lieu </Text>
          {scene.location || '—'}
          <Text dimColor>   Ambiance </Text>
          {MOODS[scene.mood] ?? (scene.mood || '—')}
        </Text>
        {scene.hook && (
          <Text wrap="truncate-end">
            <Text dimColor>Accroche </Text>
            {scene.hook}
          </Text>
        )}
        <Text wrap="truncate-end">
          <Text dimColor>Sorties </Text>
          {scene.exits.length ? scene.exits.join(' · ') : 'aucune'}
        </Text>
        <Text bold>
          Assets
        </Text>
        {assetLines(scene).map((line, i) => (
          <Text key={`asset-${i}`} color={line.ok ? 'green' : line.todo ? 'yellow' : undefined} dimColor={!line.ok && !line.todo}>
            {line.ok ? '✓' : line.todo ? '○' : '·'} {line.text}
          </Text>
        ))}
        {scene.missing.length > 0 && <Text color="yellow">Manque à la scène : {scene.missing.join(', ')}</Text>}
      </Box>
    )

    const todo = toProduce(shown)
    const footer = (
      <Box flexDirection="column" key="footer" marginTop={1}>
        <Text bold>À produire avant la soirée{actId === ALL ? '' : ' (cet acte)'}</Text>
        {todo.length === 0 ? (
          <Text color="green">Rien : toutes les scènes ont leurs assets.</Text>
        ) : (
          todo.map((line, i) => (
            <Text key={`todo-${i}`} wrap="truncate-end">
              ○ {line}
            </Text>
          ))
        )}
        <Text dimColor wrap="truncate-start">
          {scan.path}
        </Text>
      </Box>
    )

    return (
      <Box flexDirection="column">
        {header}
        {list}
        {detail}
        {footer}
      </Box>
    )
  })
}

function badges(s: Scene): string {
  const parts: string[] = []
  parts.push(s.map ? (s.map.isFound ? 'carte' : 'carte ?') : 'sans carte')
  const tracks = s.music.chosen + s.music.toFind
  if (tracks) parts.push(`♪ ${s.music.chosen}/${tracks}`)
  if (s.opponents) parts.push(`⚔ ${s.opponents}`)
  if (s.missing.length) parts.push(`${s.missing.length} manque${s.missing.length > 1 ? 's' : ''}`)
  return parts.join(' · ')
}

type AssetLine = { text: string; ok: boolean; todo: boolean }

function assetLines(s: Scene): AssetLine[] {
  const lines: AssetLine[] = []
  if (!s.map) lines.push({ text: 'Carte : aucune (scène jouée sans grille)', ok: false, todo: false })
  else if (s.map.isFound) lines.push({ text: `Carte : ${s.map.id}`, ok: true, todo: false })
  else lines.push({ text: `Carte : ${s.map.id} introuvable dans content/maps`, ok: false, todo: true })

  const tracks = s.music.chosen + s.music.toFind
  if (tracks === 0) lines.push({ text: 'Musique : aucune prévue', ok: false, todo: true })
  else if (s.music.toFind === 0) lines.push({ text: `Musique : ${tracks} morceau${tracks > 1 ? 'x' : ''} choisi${tracks > 1 ? 's' : ''}`, ok: true, todo: false })
  else lines.push({ text: `Musique : ${s.music.chosen} choisi${s.music.chosen > 1 ? 's' : ''}, ${s.music.toFind} à chercher sur YouTube`, ok: false, todo: true })

  lines.push(
    s.hasArt
      ? { text: 'Illustration : décrite (à générer dans Promptus)', ok: true, todo: false }
      : { text: 'Illustration : pas de description', ok: false, todo: true },
  )
  if (s.portraits.total > 0) {
    const all = s.portraits.described === s.portraits.total
    lines.push({ text: `Portraits des PNJ : ${s.portraits.described}/${s.portraits.total} décrits`, ok: all, todo: !all })
  }
  if (s.opponents > 0) lines.push({ text: `Combat : ${s.opponents} adversaire${s.opponents > 1 ? 's' : ''}`, ok: true, todo: false })
  return lines
}

function toProduce(scenes: Scene[]): string[] {
  const out: string[] = []
  const say = (list: Scene[], label: string) => {
    if (list.length) out.push(`${label} : ${list.map(s => s.title).join(', ')}`)
  }
  const tracks = scenes.reduce((n, s) => n + s.music.toFind, 0)
  if (tracks) out.push(`${tracks} morceau${tracks > 1 ? 'x' : ''} à chercher sur YouTube`)
  say(scenes.filter(s => s.music.chosen + s.music.toFind === 0), 'Musique à prévoir')
  say(scenes.filter(s => s.map && !s.map.isFound), 'Carte introuvable')
  say(scenes.filter(s => !s.hasArt), 'Illustration à décrire')
  say(scenes.filter(s => s.portraits.described < s.portraits.total), 'Portraits à décrire')
  say(scenes.filter(s => s.missing.length > 0), 'Scènes incomplètes')
  return out
}
