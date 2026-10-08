import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { Decision, GmRequest, LiveScreen } from '@/lib/evening'
import { cn } from '@/lib/utils'
import { BigKey, Btn, Panel, field, useTouchScreen } from './ui'

/**
 * The players' requests as cards (gm/run-live-screen): what they play
 * and say, past rulings for the same situation, and the three answers —
 * yes, no with a word, or a check (ability and difficulty) the player
 * then rolls on their phone. Answered ones show the server's roll.
 * On a tablet the request is answered with the thumb: the ability in big
 * keys, then one key per difficulty of the rules sends the check.
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
  const touch = useTouchScreen()
  if (touch) {
    return (
      <article className="flex flex-col gap-3 rounded-2xl bg-ivory p-4 text-ink shadow-ivory-flat">
        <span className="type-label text-ink-soft">
          {t('gmLive.requests.asks', { who: r.characterName || r.nickname, card: r.cardName ?? t('evening.hand.other') })}
        </span>
        {r.text && <q className="type-narration text-[22px] leading-snug [quotes:none]">{r.text}</q>}
        {r.rulings.length > 0 && (
          <p className="text-caption text-ink-soft">
            {t('gmLive.requests.ruling', {
              situation: r.rulings[0].situation,
              ability: abilities.find((a) => a[0] === r.rulings[0].ability)?.[1] ?? r.rulings[0].ability,
              difficulty: r.rulings[0].difficulty,
            })}
          </p>
        )}
        <div className="flex flex-wrap gap-2" role="radiogroup" aria-label={t('gmLive.requests.ability')}>
          {abilities.map(([id, name]) => (
            <button
              key={id}
              type="button"
              role="radio"
              aria-checked={ability === id}
              className={cn(
                'min-h-12 rounded-button border-2 px-4 text-body font-bold',
                ability === id ? 'border-ink bg-ink text-ivory' : 'border-ink/30 bg-white text-ink',
              )}
              onClick={() => setAbility(id)}
            >
              {name}
            </button>
          ))}
        </div>
        <span className="type-label text-ink-soft">{t('gmLive.requests.thumbDifficulty')}</span>
        <div className="grid grid-cols-[repeat(auto-fit,minmax(110px,1fr))] gap-2">
          {difficulties.map(([id, name, value]) => (
            <button
              key={id}
              type="button"
              aria-label={t('gmLive.requests.checkAt', { name, value })}
              className="flex min-h-[72px] flex-col items-center justify-center rounded-xl border-2 border-ink bg-white text-caption font-semibold text-ink-soft"
              onClick={() => onDecide(r.id, { kind: 'check', ability, difficulty: id })}
            >
              <b className="text-[26px] leading-none text-ink">{value}</b>
              {name}
            </button>
          ))}
        </div>
        <input
          className="min-h-12 rounded-button border border-ink/30 bg-white px-3 text-body text-ink"
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          maxLength={300}
          placeholder={t('gmLive.requests.reason')}
          aria-label={t('gmLive.requests.reason')}
        />
        <div className="grid grid-cols-2 gap-2">
          <BigKey className="border-dashed border-ink/40 bg-transparent text-ink" onClick={() => onDecide(r.id, { kind: 'accept', reason })}>
            {t('gmLive.requests.yesNoRoll')}
          </BigKey>
          <BigKey
            className="border-dashed border-ink/40 bg-transparent text-ink"
            // The player reads why; a thumb has no time to write it.
            onClick={() => onDecide(r.id, { kind: 'refuse', reason: reason.trim() || t('gmLive.requests.nothingHere') })}
          >
            {t('gmLive.requests.no')}
          </BigKey>
        </div>
      </article>
    )
  }
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
