import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { CardButton } from '@/components/game/CardButton'
import { StatusBanner, type BannerTone } from '@/components/game/StatusBanner'
import { Sprite } from '@/features/sprites/Sprite'
import { ApiError } from '@/lib/api'
import {
  fetchReview,
  returnCharacter,
  validateCharacter,
  type Change,
  type CharacterReview as Review,
} from '@/lib/table'
import { cn } from '@/lib/utils'
import { BackstoryText } from './BackstoryText'

type State =
  | { kind: 'loading' }
  | { kind: 'error' }
  | { kind: 'ready'; review: Review }

type Problem = 'changed' | 'notSubmitted' | 'error' | 'noteRequired'

/** Glyphs, not words. */
const OK_MARK = '✓'
const FLAG_MARK = '!'

/** Longest value shown in a change line, in characters. */
const VALUE_MAX = 80

/**
 * The GM reviews one character (board « Inviter », moments 3 to 5): the
 * sheet as the player will see it in play, the rule system's checks —
 * they flag, never block —, and the GM's decision: validate, or return
 * it with a word. Once the player sends it again, only what changed
 * since the last decision is listed.
 *
 * `refreshKey` moves when the live channel says this character changed.
 * Mount it keyed by `characterId`.
 */
export function CharacterReview({
  campaignId,
  characterId,
  refreshKey,
  onDecided,
}: {
  campaignId: string
  characterId: string
  refreshKey: number
  /** The decision went through: the table refreshes. */
  onDecided: () => void
}) {
  const { t } = useTranslation()
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [note, setNote] = useState('')
  const [problem, setProblem] = useState<Problem | null>(null)
  const [busy, setBusy] = useState(false)
  // The note is drafted once per version of the sheet; a GM's edits
  // survive refetches of the same version.
  const draftedFor = useRef<string | null>(null)

  const show = useCallback((next: State) => {
    setState(next)
    if (next.kind === 'ready' && draftedFor.current !== next.review.updatedAt) {
      draftedFor.current = next.review.updatedAt
      setNote(proposedNote(next.review))
    }
  }, [])

  const load = useCallback(
    () =>
      fetchReview(campaignId, characterId).then(
        (review): State => ({ kind: 'ready', review }),
        (): State => ({ kind: 'error' }),
      ),
    [campaignId, characterId],
  )

  // One character per mount: the page keys this component by its id.
  useEffect(() => {
    let live = true
    void load().then((next) => {
      if (live) show(next)
    })
    return () => {
      live = false
    }
  }, [load, show, refreshKey])

  if (state.kind === 'loading') return <p role="status">{t('gm.table.loading')}</p>
  if (state.kind === 'error') return <p role="alert">{t('gm.table.error')}</p>
  const { review } = state
  const name = review.sheet.name || t('gm.review.unnamed')
  const flagged = new Set(review.checks.map((c) => c.path))
  const moved = new Set(review.changes.map((c) => c.path))
  const resubmitted = review.status === 'submitted' && review.reviewedSheet !== null
  const abilityName = (id: string) => review.abilities.find((a) => a.id === id)?.name ?? id

  async function decide(kind: 'validate' | 'return') {
    setProblem(null)
    if (kind === 'return' && note.trim() === '') {
      setProblem('noteRequired')
      return
    }
    setBusy(true)
    try {
      if (kind === 'validate') await validateCharacter(campaignId, characterId, review.updatedAt)
      else await returnCharacter(campaignId, characterId, review.updatedAt, note.trim())
      onDecided()
      show(await load())
    } catch (err) {
      if (err instanceof ApiError && err.code === 'CHARACTER_CHANGED') setProblem('changed')
      else if (err instanceof ApiError && err.code === 'CHARACTER_NOT_SUBMITTED') setProblem('notSubmitted')
      else setProblem('error')
      if (err instanceof ApiError && err.status === 409) show(await load())
    } finally {
      setBusy(false)
    }
  }

  const banner = bannerFor(review, name, resubmitted, t)

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <div className="grid flex-1 gap-4 xl:grid-cols-[1fr_340px]">
        <article className="flex flex-col gap-4" aria-label={t('gm.review.sheetOf', { name })}>
          <div className="grid gap-5 sm:grid-cols-[190px_1fr]">
            <div className="grid h-[220px] place-items-end justify-center rounded-[14px] border border-line bg-surface pb-3.5">
              {review.sheet.look ? (
                <Sprite look={review.sheet.look} scale={5} label={name} />
              ) : (
                <span className="mb-6 text-caption text-mute">{t('gm.review.noLook')}</span>
              )}
            </div>
            <div className="flex flex-col gap-3">
              <div className="flex flex-wrap items-baseline gap-3.5">
                <h2 className="type-title text-heading">{name}</h2>
                <span className="type-label">
                  {review.className ?? review.sheet.classId ?? t('gm.review.noClass')} · {review.nickname}
                </span>
              </div>
              {review.abilities.length > 0 && (
                <dl className="grid grid-cols-3 gap-2 sm:grid-cols-6">
                  {review.abilities.map((a) => {
                    const path = `abilities.${a.id}`
                    const value = review.sheet.abilities?.[a.id]
                    return (
                      <div
                        key={a.id}
                        title={a.name}
                        data-flag={flagged.has(path) ? 'bad' : moved.has(path) ? 'fix' : undefined}
                        className={cn(
                          'flex flex-col items-center gap-0.5 rounded-[10px] border border-line bg-well py-2',
                          flagged.has(path) && 'border-[1.5px] border-stat-atk text-stat-atk',
                          !flagged.has(path) && moved.has(path) && 'border-[1.5px] border-chalk bg-surface-raised',
                        )}
                      >
                        <dt className="text-[10px] font-bold tracking-[0.14em] text-mute">{a.id}</dt>
                        <dd className="text-[20px] font-semibold">{value ?? '—'}</dd>
                      </div>
                    )
                  })}
                </dl>
              )}
              {review.rulesName && (
                <p className="text-caption text-mute">{t('gm.review.rules', { rules: review.rulesName })}</p>
              )}
            </div>
          </div>
          <Writing label={t('gm.review.backstory')}>
            <BackstoryText backstory={review.sheet.backstory} />
          </Writing>
          {review.sheet.appearance && <Writing label={t('gm.review.appearance')}>{review.sheet.appearance}</Writing>}
        </article>

        <aside className="surface-slab flex flex-col gap-2 self-start p-3.5" aria-label={t('gm.review.checkTitle')}>
          <div className="flex items-baseline justify-between">
            <h3 className="type-label text-chalk">{t('gm.review.checkTitle')}</h3>
            <span className="type-label">{name}</span>
          </div>
          {resubmitted && (
            <>
              <h4 className="type-label mt-1">{t('gm.review.changesTitle')}</h4>
              {review.changes.length === 0 ? (
                <Check ok>{t('gm.review.nothingChanged')}</Check>
              ) : (
                <ul className="flex flex-col gap-1.5">
                  {review.changes.map((c) => (
                    <li key={c.path}>
                      <Check ok>{describeChange(c, abilityName, t)}</Check>
                    </li>
                  ))}
                </ul>
              )}
              <h4 className="type-label mt-1">{t('gm.review.rulesTitle')}</h4>
            </>
          )}
          {review.checks.length === 0 ? (
            <Check ok>{t('gm.review.allGood')}</Check>
          ) : (
            <ul className="flex flex-col gap-1.5">
              {review.checks.map((c) => (
                <li key={`${c.code}:${c.path}`}>
                  <Check ok={false}>{c.message ?? c.code}</Check>
                </li>
              ))}
            </ul>
          )}

          {review.status === 'submitted' && (
            <label className="mt-2 flex flex-col gap-1.5 rounded-xl bg-ivory p-3 text-ink shadow-[0_3px_0_var(--color-ivory-edge)]">
              <span className="type-label text-ink-soft">{t('gm.review.noteLabel', { nickname: review.nickname })}</span>
              <textarea
                className="min-h-28 resize-y bg-transparent text-body leading-normal text-ink outline-none"
                value={note}
                onChange={(e) => setNote(e.target.value)}
                placeholder={t('gm.review.notePlaceholder')}
              />
            </label>
          )}
          {review.status === 'returned' && review.gmNote && (
            <div className="mt-2 rounded-xl bg-ivory p-3 text-body text-ink shadow-[0_3px_0_var(--color-ivory-edge)]">
              <span className="type-label block text-ink-soft">{t('gm.review.sentTo', { nickname: review.nickname })}</span>
              {review.gmNote}
            </div>
          )}
        </aside>
      </div>

      {problem && <p role="alert">{t(`gm.review.problem.${problem}`)}</p>}

      <footer className="flex flex-wrap items-center gap-3.5">
        <StatusBanner tone={banner.tone} className="min-w-[240px] flex-1">
          {banner.text}
        </StatusBanner>
        {review.status === 'submitted' && (
          <div className="flex w-full max-w-[460px] flex-col gap-2">
            {review.checks.length === 0 ? (
              <>
                <CardButton
                  title={t('gm.review.validate', { name })}
                  subtitle={t('gm.review.validateSub', { nickname: review.nickname })}
                  disabled={busy}
                  onClick={() => void decide('validate')}
                />
                <CardButton
                  variant="dark"
                  size="small"
                  title={t('gm.review.returnAnyway')}
                  disabled={busy}
                  onClick={() => void decide('return')}
                />
              </>
            ) : (
              <>
                <CardButton
                  title={t('gm.review.return', { nickname: review.nickname })}
                  subtitle={t('gm.review.returnSub')}
                  disabled={busy}
                  onClick={() => void decide('return')}
                />
                <CardButton
                  variant="dark"
                  size="small"
                  title={t('gm.review.validateAnyway')}
                  disabled={busy}
                  onClick={() => void decide('validate')}
                />
              </>
            )}
          </div>
        )}
      </footer>
    </div>
  )
}

function Writing({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="rounded-xl border border-line-strong bg-well px-4 py-3.5 text-body leading-relaxed text-chalk-soft">
      <span className="type-label mb-2 block">{label}</span>
      {children}
    </div>
  )
}

function Check({ ok, children }: { ok: boolean; children: ReactNode }) {
  return (
    <p
      className={cn(
        'flex items-start gap-2.5 rounded-[10px] bg-well px-3 py-2.5 text-[13px] leading-snug',
        !ok && 'border-[1.5px] border-stat-atk',
      )}
    >
      <i
        aria-hidden
        className={cn(
          'mt-px grid size-[18px] flex-none place-items-center rounded-[5px] text-[11px] font-extrabold not-italic text-ink',
          ok ? 'bg-ivory' : 'bg-stat-atk',
        )}
      >
        {ok ? OK_MARK : FLAG_MARK}
      </i>
      <span>{children}</span>
    </p>
  )
}

/** The word the review proposes when the rules flag something: their messages, for the GM to rewrite. */
function proposedNote(review: Review): string {
  if (review.status !== 'submitted') return ''
  return review.checks
    .map((c) => c.message)
    .filter(Boolean)
    .join(' ')
}

type T = ReturnType<typeof useTranslation>['t']

function bannerFor(review: Review, name: string, resubmitted: boolean, t: T): { tone: BannerTone; text: string } {
  const nickname = review.nickname
  switch (review.status) {
    case 'draft':
      return { tone: 'wait', text: t('gm.review.banner.draft', { nickname }) }
    case 'returned':
      return { tone: 'wait', text: t('gm.review.banner.returned', { nickname }) }
    case 'validated':
      return { tone: 'you', text: t('gm.review.banner.validated', { name }) }
    case 'submitted':
      if (review.checks.length > 0) {
        return { tone: 'warn', text: t('gm.review.banner.flagged', { name, count: review.checks.length }) }
      }
      return {
        tone: 'you',
        text: resubmitted ? t('gm.review.banner.corrected', { nickname, name }) : t('gm.review.banner.clean', { name }),
      }
  }
}

function formatValue(v: unknown, t: T): string {
  if (v === null || v === undefined) return t('gm.review.empty')
  if (typeof v === 'string') return v.length > VALUE_MAX ? `${v.slice(0, VALUE_MAX)}…` : v
  if (typeof v === 'number' || typeof v === 'boolean') return String(v)
  return t('gm.review.modified')
}

/** « Force : 18 → 17 », « Son histoire : … → … ». */
function describeChange(c: Change, abilityName: (id: string) => string, t: T): string {
  const known: Record<string, string> = {
    name: t('gm.review.field.name'),
    classId: t('gm.review.field.classId'),
    appearance: t('gm.review.field.appearance'),
    backstory: t('gm.review.field.backstory'),
    'backstory.origin': t('creator.story.origin'),
    'backstory.loss': t('creator.story.loss'),
    'backstory.quest': t('creator.story.quest'),
    'backstory.text': t('gm.review.field.backstory'),
  }
  const label = c.path.startsWith('abilities.')
    ? abilityName(c.path.slice('abilities.'.length))
    : c.path.startsWith('look.')
      ? t('gm.review.field.look')
      : (known[c.path] ?? c.path)
  return t('gm.review.change', { label, before: formatValue(c.before, t), after: formatValue(c.after, t) })
}
