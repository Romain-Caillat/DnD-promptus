import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { importCampaign, listCampaigns, type CampaignSummary } from '@/lib/table'

/**
 * The GM's campaigns, each opening its table page, and a YAML import to
 * bring one in. The full list, creation and settings belong to
 * `campaign/list-campaigns`.
 */
export function CampaignsPanel() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [campaigns, setCampaigns] = useState<CampaignSummary[] | null>(null)
  const [error, setError] = useState<'gm.campaigns.error' | 'gm.campaigns.importError' | null>(null)
  const [importing, setImporting] = useState(false)

  useEffect(() => {
    let live = true
    listCampaigns().then(
      (list) => {
        if (live) setCampaigns(list)
      },
      () => {
        if (live) setError('gm.campaigns.error')
      },
    )
    return () => {
      live = false
    }
  }, [])

  async function importFile(file: File) {
    setImporting(true)
    setError(null)
    try {
      const created = await importCampaign(await file.text())
      navigate(`/campagnes/${encodeURIComponent(created.id)}`)
    } catch {
      setError('gm.campaigns.importError')
      setImporting(false)
    }
  }

  return (
    <section className="flex flex-col gap-4">
      <h2 className="text-lg font-semibold">{t('gm.campaigns.title')}</h2>
      {campaigns && campaigns.length === 0 && (
        <p className="text-sm text-muted-foreground">{t('gm.campaigns.none')}</p>
      )}
      {campaigns && campaigns.length > 0 && (
        <ul className="flex flex-col gap-2">
          {campaigns.map((c) => (
            <li key={c.id}>
              <Link className="underline underline-offset-4" to={`/campagnes/${encodeURIComponent(c.id)}`}>
                {c.title}
              </Link>
            </li>
          ))}
        </ul>
      )}
      <label className="flex flex-col gap-1 text-sm">
        <span>{importing ? t('gm.campaigns.importing') : t('gm.campaigns.import')}</span>
        <input
          type="file"
          accept=".yaml,.yml,text/yaml"
          disabled={importing}
          onChange={(e) => {
            const file = e.currentTarget.files?.[0]
            if (file) void importFile(file)
          }}
        />
      </label>
      {error && <p role="alert">{t(error)}</p>}
    </section>
  )
}
