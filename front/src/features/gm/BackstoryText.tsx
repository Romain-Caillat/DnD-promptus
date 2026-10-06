import { useTranslation } from 'react-i18next'
import type { Backstory } from '@/lib/play'

const QUESTIONS = ['origin', 'loss', 'quest'] as const

/**
 * A player's backstory as the creator asks it (board « Créer », moment
 * 7): the three answers under their questions, then the paragraph. What
 * the GM draws secret hooks from.
 */
export function BackstoryText({ backstory }: { backstory: Backstory | undefined }) {
  const { t } = useTranslation()
  const answers = QUESTIONS.filter((q) => backstory?.[q])
  if (!backstory || (answers.length === 0 && !backstory.text)) {
    return <span className="text-mute">{t('gm.review.noBackstory')}</span>
  }
  return (
    <div className="flex flex-col gap-2">
      {answers.length > 0 && (
        <dl className="flex flex-col gap-1.5">
          {answers.map((q) => (
            <div key={q}>
              <dt className="text-caption text-mute">{t(`creator.story.${q}`)}</dt>
              <dd>{backstory[q]}</dd>
            </div>
          ))}
        </dl>
      )}
      {backstory.text && <p>{backstory.text}</p>}
    </div>
  )
}
