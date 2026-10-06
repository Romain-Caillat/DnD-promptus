import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Decision, GmRequest, LiveScreen } from '@/lib/evening'
import { Btn, Panel, field } from './ui'

/**
 * The players' requests as cards (gm/run-live-screen): what they play
 * and say, past rulings for the same situation, and the three answers —
 * yes, no with a word, or a check (ability and difficulty) the player
 * then rolls on their phone. Answered ones show the server's roll.
 */
export function RequestsPanel({
  screen,
  onDecide,
}: {
  screen: LiveScreen
  onDecide: (request: string, d: Decision) => void
}) {
  const { t } = useTranslation()
  const open = screen.requests.filter((r) => r.status === 'pending')
  const done = screen.requests.filter((r) => r.status !== 'pending' && r.status !== 'withdrawn').slice(-5).reverse()
  return (
    <Panel title={t('gmLive.requests.title', { count: open.length })}>
      {open.length === 0 && <p className="text-caption text-mute">{t('gmLive.requests.none')}</p>}
      {open.map((r) => (
        <PendingRequest key={r.id} r={r} screen={screen} onDecide={onDecide} />
      ))}
      {done.map((r) => (
        <p key={r.id} className="text-caption text-chalk-soft">
          {t('gmLive.requests.done', {
            who: r.characterName || r.nickname,
            card: r.cardName ?? t('evening.hand.other'),
            status: t(`evening.requests.status.${r.status}`),
          })}
          {r.roll && ` · ${t('gmLive.requests.rolled', { natural: r.roll.natural, total: r.roll.total })}`}
          {r.contested && ` · ${t('gmLive.requests.contested')}`}
        </p>
      ))}
    </Panel>
  )
}

function PendingRequest({
  r,
  screen,
  onDecide,
}: {
  r: GmRequest
  screen: LiveScreen
  onDecide: (request: string, d: Decision) => void
}) {
  const { t } = useTranslation()
  const abilities = screen.rules?.abilities ?? []
  const difficulties = screen.rules?.difficulties ?? []
  const [reason, setReason] = useState('')
  const [ability, setAbility] = useState(r.card.kind === 'ability' ? r.card.ability : (abilities[0]?.[0] ?? ''))
  const [difficulty, setDifficulty] = useState(difficulties[Math.floor(difficulties.length / 2)]?.[0] ?? '')
  return (
    <article className="flex flex-col gap-2 rounded-lg border border-ivory/60 p-2.5">
      <div className="flex items-baseline justify-between gap-2">
        <span className="text-body font-bold">{r.characterName || r.nickname}</span>
        <span className="type-label">{r.cardName ?? t('evening.hand.other')}</span>
      </div>
      {r.text && <p className="text-body">{r.text}</p>}
      {r.rulings.length > 0 && (
        <p className="text-caption text-mute-soft">
          {t('gmLive.requests.ruling', {
            situation: r.rulings[0].situation,
            ability: abilities.find((a) => a[0] === r.rulings[0].ability)?.[1] ?? r.rulings[0].ability,
            difficulty: r.rulings[0].difficulty,
          })}
        </p>
      )}
      <input
        className={field}
        value={reason}
        onChange={(e) => setReason(e.target.value)}
        maxLength={300}
        placeholder={t('gmLive.requests.reason')}
        aria-label={t('gmLive.requests.reason')}
      />
      <div className="flex flex-wrap items-center gap-1.5">
        <Btn onClick={() => onDecide(r.id, { kind: 'accept', reason })}>{t('gmLive.requests.accept')}</Btn>
        <Btn disabled={!reason.trim()} onClick={() => onDecide(r.id, { kind: 'refuse', reason })}>
          {t('gmLive.requests.refuse')}
        </Btn>
        <select className={field} value={ability} onChange={(e) => setAbility(e.target.value)} aria-label={t('gmLive.requests.ability')}>
          {abilities.map(([id, name]) => (
            <option key={id} value={id}>
              {name}
            </option>
          ))}
        </select>
        <select
          className={field}
          value={difficulty}
          onChange={(e) => setDifficulty(e.target.value)}
          aria-label={t('gmLive.requests.difficulty')}
        >
          {difficulties.map(([id, name, value]) => (
            <option key={id} value={id}>
              {t('gmLive.requests.difficultyOption', { name, value })}
            </option>
          ))}
        </select>
        <Btn main onClick={() => onDecide(r.id, { kind: 'check', ability, difficulty })}>
          {t('gmLive.requests.check')}
        </Btn>
      </div>
    </article>
  )
}
