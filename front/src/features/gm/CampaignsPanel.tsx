import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import {
  importCampaign,
  listCampaigns,
  listRulePresets,
  presetOf,
  type CampaignSummary,
  type RulePreset,
} from '@/lib/campaigns'
import { cn } from '@/lib/utils'

const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long' })

type ListState =
  | { kind: 'loading' }
  | { kind: 'error' }
  | { kind: 'ready'; campaigns: CampaignSummary[]; presets: RulePreset[] }

/**
 * The GM's campaigns as cards (board « Préparer », moment 1): the one
 * worked on last in ivory, the others in black, then « Nouvelle
 * campagne » and the YAML import; archived campaigns below. Each card
 * reopens its campaign page.
 */
export function CampaignsPanel({ gmName }: { gmName: string }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [state, setState] = useState<ListState>({ kind: 'loading' })
  const [importError, setImportError] = useState(false)
  const [importing, setImporting] = useState(false)

  useEffect(() => {
    let live = true
    // The presets only name the rule system on a card: without them the
    // list still shows.
    Promise.all([listCampaigns(), listRulePresets().catch(() => [])]).then(
      ([campaigns, presets]) => {
        if (live) setState({ kind: 'ready', campaigns, presets })
      },
      () => {
        if (live) setState({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [])

  async function importFile(file: File) {
    setImporting(true)
    setImportError(false)
    try {
      const created = await importCampaign(await file.text())
      navigate(`/campagnes/${encodeURIComponent(created.id)}`)
    } catch {
      setImportError(true)
      setImporting(false)
    }
  }

  const active = state.kind === 'ready' ? state.campaigns.filter((c) => !c.archivedAt) : []
  const archived = state.kind === 'ready' ? state.campaigns.filter((c) => c.archivedAt) : []
  const presets = state.kind === 'ready' ? state.presets : []

  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-wrap items-baseline gap-4">
        <h1 className="type-title text-heading">{t('gm.campaigns.title')}</h1>
        <span className="type-label">{t('gm.campaigns.kicker', { name: gmName })}</span>
      </div>
      {state.kind === 'loading' && <p role="status">{t('gm.home.loading')}</p>}
      {state.kind === 'error' && <p role="alert">{t('gm.campaigns.error')}</p>}
      {state.kind === 'ready' && active.length === 0 && (
        <p className="text-body text-mute-soft">{t('gm.campaigns.none')}</p>
      )}

      <ul className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-5">
        {active.map((c, i) => (
          <li key={c.id} className="flex">
            <CampaignCard campaign={c} preset={presetOf(presets, c.rules)} ivory={i === 0} />
          </li>
        ))}
        <li className="flex">
          <Link
            to="/campagnes/nouvelle"
            className="flex min-h-56 flex-1 flex-col items-center justify-center gap-2 rounded-panel border-2 border-dashed border-line-dashed p-4 text-center text-chalk-soft transition-colors hover:border-chalk hover:text-chalk"
          >
            <span aria-hidden className="type-title text-[56px] leading-none">
              +
            </span>
            <span className="type-title text-[16px]">{t('gm.campaigns.new')}</span>
            <span className="max-w-56 text-caption text-mute">{t('gm.campaigns.newHint')}</span>
          </Link>
        </li>
        <li className="flex">
          <label className="flex min-h-56 flex-1 cursor-pointer flex-col items-center justify-center gap-2 rounded-panel border border-line p-4 text-center text-mute-soft focus-within:border-chalk">
            <span className="type-title text-[16px] text-chalk-soft">
              {importing ? t('gm.campaigns.importing') : t('gm.campaigns.import')}
            </span>
            <span className="max-w-56 text-caption text-mute">{t('gm.campaigns.importHint')}</span>
            <input
              className="sr-only"
              type="file"
              accept=".yaml,.yml,text/yaml"
              disabled={importing}
              onChange={(e) => {
                const file = e.currentTarget.files?.[0]
                if (file) void importFile(file)
              }}
            />
          </label>
        </li>
      </ul>
      {importError && <p role="alert">{t('gm.campaigns.importError')}</p>}

      {archived.length > 0 && (
        <section className="flex flex-col gap-3">
          <div className="flex flex-col gap-1">
            <h2 className="type-label text-chalk">{t('gm.campaigns.archived')}</h2>
            <p className="text-caption text-mute">{t('gm.campaigns.archivedHint')}</p>
          </div>
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-5">
            {archived.map((c) => (
              <li key={c.id} className="flex">
                <CampaignCard campaign={c} preset={presetOf(presets, c.rules)} ivory={false} />
              </li>
            ))}
          </ul>
        </section>
      )}
    </section>
  )
}

function CampaignCard({
  campaign,
  preset,
  ivory,
}: {
  campaign: CampaignSummary
  preset: RulePreset | undefined
  ivory: boolean
}) {
  const { t } = useTranslation()
  const status = campaign.archivedAt ? t('gm.campaigns.status.archived') : t('gm.campaigns.status.prep')
  const kicker = campaign.world ? `${campaign.world} · ${status}` : status
  return (
    <Link
      to={`/campagnes/${encodeURIComponent(campaign.id)}`}
      className={cn('button-card w-full', !ivory && 'button-card-dark', campaign.archivedAt && 'opacity-70')}
    >
      {/* `button-card` (a pixel key) sets `display: block`: the inner span carries the layout. */}
      <span className="card-frame flex min-h-52 flex-col gap-2 px-3 py-2.5">
        <span className="type-label line-clamp-2 text-(--sub-color)">{kicker}</span>
        <span className="type-title text-[16px] leading-tight">{campaign.title}</span>
        <span className="mt-auto flex flex-col text-caption text-(--sub-color)">
          <span>{preset ? preset.name : t('gm.campaigns.rulesRef', { ...campaign.rules })}</span>
          <span>
            {t('gm.campaigns.seated', { count: campaign.playersSeated, planned: campaign.playerCount })}
          </span>
          <span>{t('gm.campaigns.lastActivity', { date: dateFormat.format(new Date(campaign.lastActivityAt)) })}</span>
        </span>
      </span>
    </Link>
  )
}
