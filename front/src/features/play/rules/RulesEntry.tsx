import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { CardButton } from '@/components/game/CardButton'
import { fetchRules, rulesPath } from '@/lib/rules'

/**
 * The way to the rules page from a player's home. When the rules
 * changed since the version the player last read, it says so and
 * breathes until they read it (`player/read-the-rules`).
 */
export function RulesEntry({ campaignId }: { campaignId: string }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [changed, setChanged] = useState(false)

  useEffect(() => {
    let live = true
    fetchRules(campaignId).then(
      (rules) => {
        if (live) setChanged(rules.changes !== null)
      },
      // The button still opens the page, which says what went wrong.
      () => {},
    )
    return () => {
      live = false
    }
  }, [campaignId])

  return (
    <CardButton
      variant={changed ? 'ivory' : 'dark'}
      sheen={changed}
      icon="eye"
      title={t(changed ? 'rules.entry.changed' : 'rules.entry.title')}
      subtitle={t(changed ? 'rules.entry.changedHint' : 'rules.entry.hint')}
      onClick={() => navigate(rulesPath(campaignId))}
    />
  )
}
