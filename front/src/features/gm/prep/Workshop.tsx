import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Btn, Panel, field } from '@/features/gm/live/ui'
import { ApiError } from '@/lib/api'
import {
  askWorkshop,
  decideProposal,
  entities,
  listProposals,
  nameOf,
  type Change,
  type Proposal,
  type ReviewCampaign,
  type Story,
  type WorkshopAsk,
} from '@/lib/prep'
import { cn } from '@/lib/utils'

/** The quick requests of the board's workshop. */
const QUICK = ['clues', 'threat', 'scene'] as const

/** The workshop's proposals and the GM's gestures on them. */
export interface WorkshopState {
  proposals: Proposal[]
  busy: boolean
  error: string | null
  send: (input: WorkshopAsk) => Promise<boolean>
  decide: (p: Proposal, accept: boolean) => Promise<void>
}

/**
 * The co-GM's workshop for `campaignId`: its proposals, reloaded when
 * `version` moves, and the GM's requests and decisions. An accepted
 * proposal hands the campaign it produced to `onCampaign`.
 */
export function useWorkshop(
  campaignId: string,
  version: number,
  onCampaign: (c: ReviewCampaign) => void,
): WorkshopState {
  const [proposals, setProposals] = useState<Proposal[]>([])
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const latest = useRef(0)

  useEffect(() => {
    const request = ++latest.current
    listProposals(campaignId).then(
      (list) => {
        if (request === latest.current) setProposals(list)
      },
      () => {
        // Kept as it was; the next change tries again.
      },
    )
  }, [campaignId, version])

  async function send(input: WorkshopAsk): Promise<boolean> {
    setBusy(true)
    setError(null)
    try {
      const p = await askWorkshop(campaignId, input)
      setProposals((cur) => [p, ...cur.filter((x) => x.id !== p.id)])
      return true
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'action')
      return false
    } finally {
      setBusy(false)
    }
  }

  async function decide(p: Proposal, accept: boolean) {
    setBusy(true)
    setError(null)
    try {
      const r = await decideProposal(campaignId, p.id, accept)
      setProposals((cur) => cur.map((x) => (x.id === p.id ? r.proposal : x)))
      onCampaign(r.campaign)
    } catch (err) {
      setError(err instanceof ApiError ? err.code : 'action')
    } finally {
      setBusy(false)
    }
  }

  return { proposals, busy, error, send, decide }
}

/**
 * The co-GM's workshop: the GM asks in their own words (about the scene
 * selected, or an alert of the coherence check), the co-GM proposes
 * changes, the GM accepts or rejects each proposal. Nothing changes in
 * the campaign before the GM accepts.
 */
export function Workshop({
  workshop,
  story,
  node,
}: {
  workshop: WorkshopState
  story: Story
  /** The scene the GM is looking at. */
  node: string | null
}) {
  const { t } = useTranslation()
  const [prompt, setPrompt] = useState('')
  const { proposals, busy, error, send, decide } = workshop

  async function ask(text: string) {
    if (await send({ prompt: text, node: node ?? undefined })) setPrompt('')
  }

  const sceneTitle = node ? story.nodes?.find((n) => n.id === node)?.title : undefined

  return (
    <Panel title={t('prep.workshop.title')} className="xl:sticky xl:top-4 xl:self-start">
      <p className="text-caption text-mute-soft">
        {sceneTitle ? t('prep.workshop.about', { scene: sceneTitle }) : t('prep.workshop.hint')}
      </p>
      <textarea
        aria-label={t('prep.workshop.prompt')}
        className={cn(field, 'min-h-20')}
        placeholder={t('prep.workshop.placeholder')}
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
      />
      <div className="flex flex-wrap gap-1.5">
        {QUICK.map((q) => (
          <Btn
            key={q}
            disabled={busy}
            onClick={() => void ask(t(`prep.workshop.quick.${q}.prompt`))}
          >
            {t(`prep.workshop.quick.${q}.label`)}
          </Btn>
        ))}
        <Btn
          main
          className="ml-auto"
          disabled={busy || !prompt.trim()}
          onClick={() => void ask(prompt.trim())}
        >
          {busy ? t('prep.workshop.asking') : t('prep.workshop.ask')}
        </Btn>
      </div>
      {error && (
        <p role="alert" className="text-caption text-stat-atk">
          {t(`prep.workshop.errors.${error}`, { defaultValue: t('prep.workshop.errors.action') })}
        </p>
      )}
      <ul className="flex flex-col gap-3">
        {proposals.map((p) => (
          <ProposalCard key={p.id} proposal={p} story={story} busy={busy} onDecide={(a) => void decide(p, a)} />
        ))}
      </ul>
    </Panel>
  )
}

function ProposalCard({
  proposal: p,
  story,
  busy,
  onDecide,
}: {
  proposal: Proposal
  story: Story
  busy: boolean
  onDecide: (accept: boolean) => void
}) {
  const { t } = useTranslation()
  const pending = p.status === 'pending'
  return (
    <li className="flex flex-col gap-2">
      <p className="self-end rounded-xl rounded-br-sm bg-surface px-3 py-2 text-caption">{p.prompt}</p>
      <div
        className={cn(
          'flex flex-col gap-1.5 rounded-xl p-2.5 text-caption',
          pending ? 'bg-ivory text-ink shadow-ivory-flat' : 'border border-line text-mute-soft',
        )}
      >
        {p.reply && <p>{p.reply}</p>}
        {pending && (
          <ul className="flex flex-col gap-1 border-t border-ink/30 pt-1.5">
            {p.changes.map((c, i) => (
              <li key={i}>
                <ChangeText change={c} story={story} />
              </li>
            ))}
          </ul>
        )}
        {p.dropped > 0 && <p className="opacity-70">{t('prep.workshop.dropped', { count: p.dropped })}</p>}
        {p.stale && <p className="font-semibold">{t('prep.workshop.stale')}</p>}
        {pending ? (
          <div className="flex gap-2 border-t border-ink/30 pt-1.5">
            <Btn main disabled={busy || p.stale || p.changes.length === 0} onClick={() => onDecide(true)}>
              {t('prep.workshop.accept')}
            </Btn>
            <Btn className="text-ink" disabled={busy} onClick={() => onDecide(false)}>
              {t('prep.workshop.reject')}
            </Btn>
          </div>
        ) : (
          <p className="font-semibold">{t(`prep.workshop.status.${p.status}`)}</p>
        )}
      </div>
    </li>
  )
}

const MARK = { add: '+', set: '~', remove: '−' } as const

function short(v: unknown): string {
  if (v === undefined || v === null || v === '') return '∅'
  const text = typeof v === 'string' ? v : JSON.stringify(v)
  return text.length > 80 ? `${text.slice(0, 80)}…` : text
}

/** One change of a proposal, as the board writes it. */
function ChangeText({ change: c, story }: { change: Change; story: Story }) {
  const { t } = useTranslation()
  const kind = t(`prep.workshop.kind.${c.kind}`, { defaultValue: c.kind })
  const name = c.name || c.id
  const place = c.place ? nameOf(entities(story).get(c.place)) || c.place : null
  return (
    <span>
      <b className="mr-1 font-mono">{MARK[c.op]}</b>
      {c.op === 'set'
        ? t('prep.workshop.setLine', {
            kind,
            name,
            field: t(`prep.review.field.${c.field ?? ''}`, { defaultValue: c.field ?? '' }),
            before: short(c.before),
            after: short(c.after),
          })
        : place
          ? t('prep.workshop.placedLine', { kind, name, place })
          : t('prep.workshop.line', { kind, name })}
    </span>
  )
}
