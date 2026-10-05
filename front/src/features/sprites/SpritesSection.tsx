import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { apiRequest } from '@/lib/api'
import { CONDITION_VISUALS, type CharacterLook } from './look'
import { Sprite } from './Sprite'

interface NamedLook {
  id: string
  look: CharacterLook
}

interface LookBook {
  world: string
  party: NamedLook[]
  foes: NamedLook[]
}

type State = { kind: 'loading' } | { kind: 'error' } | { kind: 'ready'; worlds: LookBook[] }

/**
 * The characters of the two witness worlds, drawn by the app from their
 * description: the six party slots facing right, their foes facing left,
 * then every condition effect on one of them.
 */
export function SpritesSection() {
  const { t } = useTranslation()
  const [state, setState] = useState<State>({ kind: 'loading' })

  useEffect(() => {
    let live = true
    apiRequest<{ worlds: LookBook[] }>('GET', '/sprites/looks')
      .then((data) => live && setState({ kind: 'ready', worlds: data.worlds }))
      .catch(() => live && setState({ kind: 'error' }))
    return () => {
      live = false
    }
  }, [])

  return (
    <section className="surface-slab flex flex-col gap-4 p-5">
      <h2 className="type-label">{t('reference.sprites.title')}</h2>
      {state.kind === 'loading' && <p className="text-caption text-mute">{t('reference.sprites.loading')}</p>}
      {state.kind === 'error' && <p className="text-caption text-mute">{t('reference.sprites.error')}</p>}
      {state.kind === 'ready' &&
        state.worlds.map((world) => (
          <div key={world.world} className="flex flex-col gap-3">
            <h3 className="type-title text-card-title">
              {isKnownWorld(world.world) ? t(`reference.sprites.worlds.${world.world}`) : world.world}
            </h3>
            <ul className="flex flex-wrap items-end gap-x-4 gap-y-3">
              {world.party.map((p) => (
                <SpriteTile key={p.id} id={p.id} look={p.look} facing="east" />
              ))}
              {world.foes.map((p) => (
                <SpriteTile key={p.id} id={p.id} look={p.look} facing="west" />
              ))}
            </ul>
          </div>
        ))}
      {state.kind === 'ready' && state.worlds[0]?.party[0] && (
        <div className="flex flex-col gap-3 border-t border-line pt-4">
          <h3 className="type-label">{t('reference.sprites.effects')}</h3>
          <ul className="flex flex-wrap items-end gap-x-6 gap-y-6 pt-4">
            {CONDITION_VISUALS.map((v) => (
              <li key={v} className="flex flex-col items-center gap-2">
                <Sprite look={state.worlds[0].party[0].look} scale={3} effects={[v]} />
                <span className="text-caption text-mute">{t(`reference.sprites.visuals.${v}`)}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
      <p className="text-caption text-mute">{t('reference.sprites.note')}</p>
    </section>
  )
}

const KNOWN_WORLDS = ['corsaires', 'brasier'] as const

function isKnownWorld(world: string): world is (typeof KNOWN_WORLDS)[number] {
  return (KNOWN_WORLDS as readonly string[]).includes(world)
}

function SpriteTile({ id, look, facing }: { id: string; look: CharacterLook; facing: 'east' | 'west' }) {
  return (
    <li className="flex flex-col items-center gap-1.5">
      <Sprite look={look} scale={4} facing={facing} label={id} />
      <code className="text-label text-mute">{id}</code>
    </li>
  )
}
