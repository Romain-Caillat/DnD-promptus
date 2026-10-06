import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useNavigate, useParams } from 'react-router'
import { CardButton } from '@/components/game/CardButton'
import { StatusBanner } from '@/components/game/StatusBanner'
import { ApiError } from '@/lib/api'
import {
  archiveCampaign,
  fetchCampaign,
  listRulePresets,
  presetOf,
  saveCampaignSettings,
  type CampaignDetail,
  type RulePreset,
} from '@/lib/campaigns'
import { IdentityFields, PitchFields, PresetStats } from './CampaignFields'
import { formOf, settingsOf, type CampaignForm, type FormError } from './campaignForm'

const dateFormat = new Intl.DateTimeFormat('fr-FR', { dateStyle: 'long' })

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; campaign: CampaignDetail }

type Notice = { kind: 'saved' } | { kind: 'error'; code: FormError | 'action' }

function noticeOf(err: unknown): Notice {
  if (err instanceof ApiError) {
    const code = (['TITLE_REQUIRED', 'INVALID_PLAYER_COUNT', 'INVALID_AI_BUDGET'] as const).find((c) => c === err.code)
    if (code) return { kind: 'error', code }
  }
  return { kind: 'error', code: 'action' }
}

/**
 * `/campagnes/:campaignId` — a campaign reopened: where it stands, its
 * settings (title, universe, pitch, players' hook, table size, AI
 * budget), its rule system, and the way to its table (invitation) and
 * to what the players see. Archiving shelves it; it can come back.
 */
export function CampaignPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [presets, setPresets] = useState<RulePreset[]>([])
  const [form, setForm] = useState<CampaignForm | null>(null)
  const [notice, setNotice] = useState<Notice | null>(null)
  const [busy, setBusy] = useState(false)
  const [confirmArchive, setConfirmArchive] = useState(false)

  useEffect(() => {
    let live = true
    fetchCampaign(campaignId).then(
      (campaign) => {
        if (!live) return
        setState({ kind: 'ready', campaign })
        setForm(formOf(campaign))
      },
      (err: unknown) => {
        if (!live) return
        if (err instanceof ApiError && err.status === 401) setState({ kind: 'signed-out' })
        else if (err instanceof ApiError && err.status === 404) setState({ kind: 'not-found' })
        else setState({ kind: 'error' })
      },
    )
    // Only names the rule system: the page works without it.
    listRulePresets().then(
      (list) => {
        if (live) setPresets(list)
      },
      () => {},
    )
    return () => {
      live = false
    }
  }, [campaignId])

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />

  if (state.kind !== 'ready' || !form) {
    return (
      <main className="surface-table flex min-h-dvh flex-col gap-4 p-6 text-chalk">
        <Link className="text-caption text-mute-soft underline underline-offset-4" to="/">
          {t('gm.campaign.back')}
        </Link>
        {state.kind === 'loading' && <p role="status">{t('gm.campaign.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('gm.campaign.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('gm.campaign.error')}</p>}
      </main>
    )
  }

  const { campaign } = state
  const preset = presetOf(presets, campaign.story.rules)
  const base = `/campagnes/${encodeURIComponent(campaign.id)}`

  async function save() {
    if (!form) return
    const settings = settingsOf(form)
    if (typeof settings === 'string') {
      setNotice({ kind: 'error', code: settings })
      return
    }
    setBusy(true)
    setNotice(null)
    try {
      const updated = await saveCampaignSettings(campaignId, settings)
      setState({ kind: 'ready', campaign: updated })
      setForm(formOf(updated))
      setNotice({ kind: 'saved' })
    } catch (err) {
      setNotice(noticeOf(err))
    } finally {
      setBusy(false)
    }
  }

  async function shelve(archived: boolean) {
    setBusy(true)
    setNotice(null)
    try {
      const updated = await archiveCampaign(campaignId, archived)
      setState({ kind: 'ready', campaign: updated })
      setConfirmArchive(false)
    } catch (err) {
      setNotice(noticeOf(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <main className="surface-table flex min-h-dvh flex-col text-chalk">
      <header className="flex flex-wrap items-center gap-4 border-b border-line px-5 py-3 text-caption text-mute-soft">
        <Link className="underline underline-offset-4" to="/">
          {t('gm.campaign.back')}
        </Link>
        <h1 className="type-title text-[15px] text-chalk">{campaign.story.title}</h1>
        <span className="rounded-md border border-line-strong px-2 py-0.5 text-[11px] font-semibold">
          {campaign.archivedAt ? t('gm.campaigns.status.archived') : t('gm.campaigns.status.prep')}
        </span>
      </header>

      <div className="flex flex-col gap-5 p-5">
        {campaign.archivedAt ? (
          <StatusBanner tone="warn">
            {t('gm.campaign.bannerArchived', { date: dateFormat.format(new Date(campaign.archivedAt)) })}
          </StatusBanner>
        ) : (
          <StatusBanner tone="listen">{t('gm.campaign.bannerPrep')}</StatusBanner>
        )}

        <div className="grid gap-5 lg:grid-cols-[1fr_360px]">
          <form
            className="surface-slab flex flex-col gap-4 p-4"
            onSubmit={(e) => {
              e.preventDefault()
              void save()
            }}
          >
            <h2 className="type-label text-chalk">{t('gm.campaign.settings')}</h2>
            <IdentityFields
              form={form}
              onChange={(patch) => {
                setForm({ ...form, ...patch })
                setNotice(null)
              }}
            />
            <PitchFields
              form={form}
              onChange={(patch) => {
                setForm({ ...form, ...patch })
                setNotice(null)
              }}
            />
            <div className="flex flex-wrap items-center justify-end gap-4">
              {notice?.kind === 'saved' && (
                <p role="status" className="text-body text-chalk-soft">
                  {t('gm.campaign.saved')}
                </p>
              )}
              {notice?.kind === 'error' && (
                <p role="alert" className="text-body text-stat-atk">
                  {notice.code === 'action' ? t('gm.campaign.actionError') : t(`gm.newCampaign.errors.${notice.code}`)}
                </p>
              )}
              <CardButton
                type="submit"
                className="max-w-[420px]"
                title={busy ? t('gm.campaign.saving') : t('gm.campaign.save')}
                subtitle={t('gm.campaign.saveSub')}
                disabled={busy}
              />
            </div>
          </form>

          <aside className="flex flex-col gap-4">
            <CardButton
              icon="arrow"
              title={t('gm.campaign.evening')}
              subtitle={t('gm.campaign.eveningSub')}
              onClick={() => navigate(`${base}/soiree`)}
            />
            <CardButton
              variant="dark"
              icon="link"
              title={t('gm.campaign.table')}
              subtitle={t('gm.campaign.tableSub')}
              onClick={() => navigate(`${base}/table`)}
            />
            <CardButton
              variant="dark"
              icon="eye"
              title={t('gm.campaign.playerView')}
              subtitle={t('gm.campaign.playerViewSub')}
              onClick={() => navigate(`${base}/vue-joueurs`)}
            />

            <section className="surface-slab flex flex-col gap-2 p-4">
              <h2 className="type-label text-chalk">{t('gm.campaign.rules')}</h2>
              {preset ? (
                <>
                  <span className="type-title text-[18px]">{preset.name}</span>
                  <PresetStats preset={preset} />
                </>
              ) : (
                <span className="text-body text-mute-soft">{t('gm.campaigns.rulesRef', { ...campaign.story.rules })}</span>
              )}
              <CardButton
                variant="dark"
                size="small"
                title={t('gm.campaign.editRules')}
                onClick={() => navigate(`${base}/regles`)}
              />
            </section>

            {campaign.archivedAt ? (
              <CardButton
                variant="dark"
                size="small"
                title={t('gm.campaign.unarchive')}
                disabled={busy}
                onClick={() => void shelve(false)}
              />
            ) : (
              <CardButton
                variant="dark"
                size="small"
                title={confirmArchive ? t('gm.campaign.confirmArchive') : t('gm.campaign.archive')}
                disabled={busy}
                onClick={() => (confirmArchive ? void shelve(true) : setConfirmArchive(true))}
              />
            )}
          </aside>
        </div>
      </div>
    </main>
  )
}
