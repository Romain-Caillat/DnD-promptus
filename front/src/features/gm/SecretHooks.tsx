import { useEffect, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { CardButton } from '@/components/game/CardButton'
import { Sprite } from '@/features/sprites/Sprite'
import { BackstoryText } from './BackstoryText'
import { ApiError } from '@/lib/api'
import type { Backstory } from '@/lib/play'
import {
  addHook,
  deleteHook,
  editHook,
  fetchHooks,
  fetchReview,
  proposeHooks,
  type HookInput,
  type HookTarget,
  type SecretHook,
  type Seat,
} from '@/lib/table'

type State =
  | { kind: 'loading' }
  | { kind: 'error' }
  | { kind: 'ready'; hooks: SecretHook[]; targets: HookTarget[] }

interface Draft {
  /** The hook being rewritten, or `null` for a new one. */
  id: string | null
  characterId: string
  title: string
  body: string
  links: string[]
}

const emptyDraft = (characterId: string): Draft => ({ id: null, characterId, title: '', body: '', links: [] })

/**
 * The secret hooks (board « Inviter », moment 6): what the GM draws from
 * the players' backstories and ties into nodes and fronts of the story.
 * GM-only — no player route reads them. Written by hand, or proposed
 * by the co-GM from the backstory (`copilot/co-write-backstory`): a
 * proposal is kept, reworked in the form, or set aside — nothing is
 * stored until the GM keeps it.
 */
export function SecretHooks({ campaignId, seats }: { campaignId: string; seats: Seat[] }) {
  const { t } = useTranslation()
  const characters = seats.flatMap((s) => (s.character ? [{ seat: s, character: s.character }] : []))
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [draft, setDraft] = useState<Draft>(() => emptyDraft(characters[0]?.character.id ?? ''))
  const [backstory, setBackstory] = useState<{ id: string; story: Backstory | undefined } | null>(null)
  const [failed, setFailed] = useState<
    'gm.hooks.titleRequired' | 'gm.table.error' | 'gm.hooks.proposeBudget' | 'gm.hooks.proposeFailed' | null
  >(null)
  const [proposals, setProposals] = useState<{ characterId: string; hooks: HookInput[]; dropped: number } | null>(
    null,
  )
  const [proposing, setProposing] = useState(false)
  const [confirming, setConfirming] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    fetchHooks(campaignId).then(
      (data) => live && setState({ kind: 'ready', ...data }),
      () => live && setState({ kind: 'error' }),
    )
    return () => {
      live = false
    }
  }, [campaignId])

  // A table that filled up after this opened: the first character.
  const characterId = draft.characterId || characters[0]?.character.id || ''
  // The backstory the hook is drawn from, next to the form.
  useEffect(() => {
    if (!characterId) return
    let live = true
    fetchReview(campaignId, characterId).then(
      (r) => live && setBackstory({ id: characterId, story: r.sheet.backstory }),
      () => live && setBackstory({ id: characterId, story: undefined }),
    )
    return () => {
      live = false
    }
  }, [campaignId, characterId])

  if (state.kind === 'loading') return <p role="status">{t('gm.table.loading')}</p>
  if (state.kind === 'error') return <p role="alert">{t('gm.table.error')}</p>
  const { hooks, targets } = state
  const complete = characters.length > 0 && characters.every((c) => c.character.status === 'validated')
  const characterName = (id: string) => {
    const found = characters.find((c) => c.character.id === id)
    return found ? found.character.name || found.seat.nickname : ''
  }
  const targetTitle = (id: string) => targets.find((x) => x.id === id)?.title ?? id

  async function save(e: FormEvent) {
    e.preventDefault()
    if (state.kind !== 'ready') return
    setFailed(null)
    const input = { title: draft.title, body: draft.body, links: draft.links }
    try {
      const saved = draft.id
        ? await editHook(campaignId, draft.id, input)
        : await addHook(campaignId, characterId, input)
      const rest = state.hooks.filter((h) => h.id !== saved.id)
      setState({ ...state, hooks: draft.id ? state.hooks.map((h) => (h.id === saved.id ? saved : h)) : [...rest, saved] })
      setDraft(emptyDraft(characterId))
    } catch (err) {
      const code = err instanceof ApiError ? err.code : ''
      setFailed(code === 'HOOK_TITLE_REQUIRED' ? 'gm.hooks.titleRequired' : 'gm.table.error')
    }
  }

  async function remove(hook: SecretHook) {
    if (state.kind !== 'ready') return
    if (confirming !== hook.id) {
      setConfirming(hook.id)
      return
    }
    setFailed(null)
    try {
      await deleteHook(campaignId, hook.id)
      setState({ ...state, hooks: state.hooks.filter((h) => h.id !== hook.id) })
      if (draft.id === hook.id) setDraft(emptyDraft(characterId))
    } catch {
      setFailed('gm.table.error')
    } finally {
      setConfirming(null)
    }
  }

  async function propose() {
    setFailed(null)
    setProposing(true)
    try {
      setProposals({ characterId, ...(await proposeHooks(campaignId, characterId)) })
    } catch (err) {
      const code = err instanceof ApiError ? err.code : ''
      setFailed(code === 'AI_BUDGET_EXCEEDED' ? 'gm.hooks.proposeBudget' : 'gm.hooks.proposeFailed')
    } finally {
      setProposing(false)
    }
  }

  /** Take one proposal out of the list. */
  const settle = (hook: HookInput) =>
    setProposals((p) => (p ? { ...p, hooks: p.hooks.filter((h) => h !== hook) } : p))

  async function keep(hook: HookInput) {
    if (state.kind !== 'ready' || !proposals) return
    setFailed(null)
    try {
      const saved = await addHook(campaignId, proposals.characterId, hook)
      setState({ ...state, hooks: [...state.hooks, saved] })
      settle(hook)
    } catch {
      setFailed('gm.table.error')
    }
  }

  const toggleLink = (id: string) =>
    setDraft((d) => ({ ...d, links: d.links.includes(id) ? d.links.filter((l) => l !== id) : [...d.links, id] }))

  return (
    <section className="flex flex-col gap-4" aria-labelledby="hooks-title">
      <div className="flex flex-wrap items-baseline gap-3.5">
        <h1 id="hooks-title" className="type-title text-heading">
          {complete ? t('gm.hooks.complete') : t('gm.hooks.title')}
        </h1>
        <span className="type-label">{t('gm.hooks.kicker')}</span>
      </div>

      {characters.length > 0 && (
        <ul className="flex flex-wrap gap-3.5">
          {characters.map(({ seat, character }) => (
            <li
              key={character.id}
              className="flex h-[190px] min-w-[150px] flex-1 flex-col items-center justify-end gap-1.5 rounded-none border border-line bg-surface pb-3"
            >
              {character.look && <Sprite look={character.look} scale={4} />}
              <span className="type-title text-[16px]">{character.name || seat.nickname}</span>
              <span className="text-[11px] text-mute">
                {[seat.nickname, character.className].filter(Boolean).join(' · ')}
              </span>
            </li>
          ))}
        </ul>
      )}

      <p className="text-caption text-mute">{t('gm.hooks.secret')}</p>

      {hooks.length === 0 ? (
        <p className="text-body text-mute">{t('gm.hooks.none')}</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {hooks.map((h) => (
            <li
              key={h.id}
              className="flex flex-col gap-1 rounded-none border border-ink bg-ivory px-3 py-2.5 text-[13px] leading-snug text-ink"
            >
              <b>{h.title}</b>
              {h.body && <span>{h.body}</span>}
              <small className="text-[11px] text-ink-soft">
                {t('gm.hooks.from', { name: characterName(h.characterId) })}
                {h.links.length > 0 && ` · → ${h.links.map(targetTitle).join(', ')}`} · {t('gm.hooks.kept')}
              </small>
              <span className="flex gap-2">
                <Button
                  size="sm"
                  variant="ghost"
                  className="text-ink"
                  onClick={() =>
                    setDraft({ id: h.id, characterId: h.characterId, title: h.title, body: h.body, links: h.links })
                  }
                >
                  {t('gm.hooks.edit')}
                </Button>
                <Button size="sm" variant="ghost" className="text-ink" onClick={() => void remove(h)}>
                  {confirming === h.id ? t('gm.hooks.confirmDelete') : t('gm.hooks.delete')}
                </Button>
              </span>
            </li>
          ))}
        </ul>
      )}

      {characters.length === 0 ? (
        <p className="text-body text-mute">{t('gm.hooks.noCharacters')}</p>
      ) : (
        <form className="surface-slab flex flex-col gap-3 p-3.5" onSubmit={(e) => void save(e)}>
          <h2 className="type-label text-chalk">{draft.id ? t('gm.hooks.editTitle') : t('gm.hooks.newTitle')}</h2>
          <label className="flex flex-col gap-1.5">
            <span className="type-label">{t('gm.hooks.character')}</span>
            <select
              className="pixel-field p-2.5 text-body"
              value={characterId}
              disabled={draft.id !== null}
              onChange={(e) => setDraft({ ...draft, characterId: e.target.value })}
            >
              {characters.map(({ seat, character }) => (
                <option key={character.id} value={character.id}>
                  {character.name || seat.nickname}
                </option>
              ))}
            </select>
          </label>
          {backstory?.id === characterId && (
            <div className="pixel-well px-4 py-3 text-body leading-relaxed text-chalk-soft">
              <span className="type-label mb-1.5 block">{t('gm.review.backstory')}</span>
              <BackstoryText backstory={backstory.story} />
            </div>
          )}
          {backstory?.id === characterId && backstory.story && (
            <Button
              type="button"
              variant="ghost"
              className="self-start"
              disabled={proposing}
              onClick={() => void propose()}
            >
              {proposing ? t('gm.hooks.proposing') : t('gm.hooks.propose')}
            </Button>
          )}
          {proposals?.characterId === characterId && (
            <div className="flex flex-col gap-2" aria-label={t('gm.hooks.proposals')}>
              <span className="type-label">{t('gm.hooks.proposals')}</span>
              {proposals.hooks.length === 0 && <p className="text-caption text-mute">{t('gm.hooks.proposalsDone')}</p>}
              {proposals.hooks.map((h, i) => (
                <article
                  key={`${i}-${h.title}`}
                  className="flex flex-col gap-1 rounded-none border border-dashed border-line-strong bg-well px-3 py-2.5 text-[13px] leading-snug"
                >
                  <b>{h.title}</b>
                  {h.body && <span>{h.body}</span>}
                  {h.links.length > 0 && (
                    <small className="text-[11px] text-mute">→ {h.links.map(targetTitle).join(', ')}</small>
                  )}
                  <span className="flex flex-wrap gap-2">
                    <Button type="button" size="sm" onClick={() => void keep(h)}>
                      {t('gm.hooks.keep')}
                    </Button>
                    <Button
                      type="button"
                      size="sm"
                      variant="ghost"
                      onClick={() => {
                        setDraft({ id: null, characterId, ...h })
                        settle(h)
                      }}
                    >
                      {t('gm.hooks.rework')}
                    </Button>
                    <Button type="button" size="sm" variant="ghost" onClick={() => settle(h)}>
                      {t('gm.hooks.discard')}
                    </Button>
                  </span>
                </article>
              ))}
              {proposals.dropped > 0 && (
                <p className="text-caption text-mute">{t('gm.hooks.dropped', { count: proposals.dropped })}</p>
              )}
            </div>
          )}
          <label className="flex flex-col gap-1.5">
            <span className="type-label">{t('gm.hooks.hookTitle')}</span>
            <input
              className="pixel-field p-2.5 text-body"
              value={draft.title}
              maxLength={120}
              onChange={(e) => setDraft({ ...draft, title: e.target.value })}
            />
          </label>
          <label className="flex flex-col gap-1.5">
            <span className="type-label">{t('gm.hooks.body')}</span>
            <textarea
              className="min-h-20 pixel-field p-2.5 text-body"
              value={draft.body}
              maxLength={2000}
              onChange={(e) => setDraft({ ...draft, body: e.target.value })}
            />
          </label>
          {targets.length > 0 && (
            <fieldset className="flex flex-col gap-1.5">
              <legend className="type-label mb-1.5">{t('gm.hooks.links')}</legend>
              <div className="flex flex-wrap gap-2">
                {targets.map((target) => (
                  <label
                    key={target.id}
                    className="flex items-center gap-2 pixel-well px-2.5 py-1.5 text-caption"
                  >
                    <input
                      type="checkbox"
                      checked={draft.links.includes(target.id)}
                      onChange={() => toggleLink(target.id)}
                    />
                    <span>
                      {target.title}
                      <span className="text-mute"> · {t(`gm.hooks.kind.${target.kind}`)}</span>
                    </span>
                  </label>
                ))}
              </div>
            </fieldset>
          )}
          {failed && <p role="alert">{t(failed)}</p>}
          <div className="flex flex-wrap items-center gap-3">
            <CardButton
              type="submit"
              width={320}
              title={draft.id ? t('gm.hooks.save') : t('gm.hooks.add')}
              subtitle={t('gm.hooks.addSub')}
            />
            {draft.id && (
              <Button variant="ghost" onClick={() => setDraft(emptyDraft(characterId))}>
                {t('gm.hooks.cancel')}
              </Button>
            )}
          </div>
        </form>
      )}
    </section>
  )
}
