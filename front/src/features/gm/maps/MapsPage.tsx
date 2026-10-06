import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, Navigate, useNavigate, useParams } from 'react-router'
import { Btn, Panel, field } from '@/features/gm/live/ui'
import { ApiError } from '@/lib/api'
import {
  createMap,
  deleteMap,
  fetchMaps,
  generateMap,
  importMap,
  type CampaignMap,
  type MapImport,
  type MapListing,
} from '@/lib/maps'
import { fetchReview, type Story } from '@/lib/prep'

type PageState =
  | { kind: 'loading' }
  | { kind: 'signed-out' }
  | { kind: 'not-found' }
  | { kind: 'error' }
  | { kind: 'ready'; listing: MapListing; story: Story }

/** An image the GM is aligning on a grid before importing it. */
interface Aligning {
  name: string
  dataUrl: string
  width: number
  height: number
}

/** The numbers that lay the grid on an image: what each sets, its label, its least value. */
const ALIGN_FIELDS = [
  ['cell', 'maps.align.cell', 8],
  ['ox', 'maps.align.offsetX', 0],
  ['oy', 'maps.align.offsetY', 0],
] as const

const UVTT = /\.(dd2vtt|uvtt|df2vtt|json)$/i

function readFile(file: File, as: 'text' | 'dataUrl'): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result))
    reader.onerror = () => reject(new Error('read'))
    if (as === 'text') reader.readAsText(file)
    else reader.readAsDataURL(file)
  })
}

/**
 * `/campagnes/:campaignId/cartes` — the campaign's own maps: a blank one
 * or a copy of the world's (maps/edit-map-gm), the model's draft for a
 * scene (maps/generate-map-llm), a Dungeondraft export or an image
 * aligned on a grid (maps/import-image-map). Each opens in the editor;
 * only a validated one reaches the table.
 */
export function MapsPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { campaignId = '' } = useParams()
  const [state, setState] = useState<PageState>({ kind: 'loading' })
  const [version, setVersion] = useState(0)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<{
    code: string
    detail: string | null
  } | null>(null)
  const [name, setName] = useState('')
  const [copy, setCopy] = useState('')
  const [node, setNode] = useState('')
  const [aligning, setAligning] = useState<Aligning | null>(null)

  useEffect(() => {
    let live = true
    Promise.all([fetchMaps(campaignId), fetchReview(campaignId)]).then(
      ([listing, review]) => {
        if (live) setState({ kind: 'ready', listing, story: review.story })
      },
      (err: unknown) => {
        if (!live) return
        if (err instanceof ApiError && err.status === 401) setState({ kind: 'signed-out' })
        else if (err instanceof ApiError && err.status === 404) setState({ kind: 'not-found' })
        else setState({ kind: 'error' })
      },
    )
    return () => {
      live = false
    }
  }, [campaignId, version])

  if (state.kind === 'signed-out') return <Navigate to="/connexion" replace />
  const back = (
    <Link className="text-caption text-mute-soft underline underline-offset-4" to={`/campagnes/${campaignId}`}>
      {t('prep.back')}
    </Link>
  )
  if (state.kind !== 'ready') {
    return (
      <main className="surface-table flex min-h-dvh flex-col gap-4 p-6 text-chalk">
        {back}
        {state.kind === 'loading' && <p role="status">{t('prep.loading')}</p>}
        {state.kind === 'not-found' && <p role="alert">{t('prep.notFound')}</p>}
        {state.kind === 'error' && <p role="alert">{t('prep.error')}</p>}
      </main>
    )
  }

  const { listing, story } = state
  const scenes = [...(story.nodes ?? [])].sort((a, b) => Number(Boolean(b.encounter)) - Number(Boolean(a.encounter)))
  const sceneTitle = (id: string | null) => scenes.find((n) => n.id === id)?.title ?? id

  async function open(run: () => Promise<CampaignMap>) {
    setBusy(true)
    setError(null)
    try {
      const made = await run()
      navigate(`/campagnes/${campaignId}/cartes/${made.map.id}`)
    } catch (err) {
      setError(err instanceof ApiError ? { code: err.code, detail: err.detail } : { code: 'action', detail: null })
      setBusy(false)
    }
  }

  async function chooseFile(file: File) {
    const base = file.name.replace(/\.[^.]+$/, '')
    if (UVTT.test(file.name)) {
      const text = await readFile(file, 'text')
      await open(() =>
        importMap(campaignId, {
          kind: 'uvtt',
          name: name.trim() || base,
          file: text,
        }),
      )
      return
    }
    const dataUrl = await readFile(file, 'dataUrl')
    const img = new Image()
    img.onload = () =>
      setAligning({
        name: name.trim() || base,
        dataUrl,
        width: img.naturalWidth,
        height: img.naturalHeight,
      })
    img.onerror = () => setError({ code: 'BAD_IMAGE', detail: null })
    img.src = dataUrl
  }

  return (
    <main className="surface-table flex min-h-dvh flex-col gap-4 p-5 text-chalk">
      {back}
      <h1 className="type-title text-[22px]">{t('maps.title')}</h1>
      {error && (
        <p role="alert" className="text-body text-stat-atk">
          {t(`maps.errors.${error.code}`, {
            defaultValue: t('prep.generate.errors.action'),
          })}
          {error.detail && error.code !== 'INVALID_BODY' && <span className="block text-caption">{error.detail}</span>}
        </p>
      )}

      <section className="flex flex-col gap-2" aria-label={t('maps.mine')}>
        {listing.maps.length === 0 && <p className="text-body text-mute-soft">{t('maps.none')}</p>}
        {listing.maps.map((m) => (
          <article key={m.map.id} className="flex flex-wrap items-center gap-3 rounded-lg border border-line p-2.5">
            <div className="flex min-w-0 flex-1 flex-col">
              <span className="text-body font-semibold">{m.map.name}</span>
              <span className="text-caption text-mute-soft">
                {t(`maps.source.${m.source}`)}
                {m.node && ` · ${sceneTitle(m.node)}`} · {m.validatedAt ? t('maps.validated') : t('maps.draft')}
              </span>
            </div>
            <Btn main onClick={() => navigate(`/campagnes/${campaignId}/cartes/${m.map.id}`)}>
              {t('maps.open')}
            </Btn>
            <Btn
              disabled={busy}
              onClick={() => {
                if (!window.confirm(t('maps.confirmDelete', { name: m.map.name }))) return
                setBusy(true)
                deleteMap(campaignId, m.map.id)
                  .then(() => setVersion((v) => v + 1))
                  .catch((err: unknown) =>
                    setError({
                      code: err instanceof ApiError ? err.code : 'action',
                      detail: null,
                    }),
                  )
                  .finally(() => setBusy(false))
              }}
            >
              {t('maps.delete')}
            </Btn>
          </article>
        ))}
      </section>

      <div className="grid gap-4 xl:grid-cols-3">
        <Panel title={t('maps.generateTitle')}>
          <select className={field} value={node} onChange={(e) => setNode(e.target.value)} aria-label={t('maps.scene')}>
            <option value="">{t('maps.scene')}</option>
            {scenes.map((n) => (
              <option key={n.id} value={n.id}>
                {n.title}
                {n.encounter ? ` — ${t('maps.fight')}` : ''}
              </option>
            ))}
          </select>
          <Btn main disabled={busy || !node} onClick={() => void open(() => generateMap(campaignId, node))}>
            {busy && node ? t('maps.generating') : t('maps.generate')}
          </Btn>
          <p className="text-caption text-mute-soft">{t('maps.generateHint')}</p>
        </Panel>

        <Panel title={t('maps.newTitle')}>
          <input
            className={field}
            value={name}
            maxLength={80}
            onChange={(e) => setName(e.target.value)}
            placeholder={t('maps.name')}
            aria-label={t('maps.name')}
          />
          <select className={field} value={copy} onChange={(e) => setCopy(e.target.value)} aria-label={t('maps.copy')}>
            <option value="">{t('maps.blank')}</option>
            {listing.world.map((w) => (
              <option key={w.id} value={w.id}>
                {t('maps.copyOf', { name: w.name })}
              </option>
            ))}
          </select>
          <Btn
            main
            disabled={busy || !name.trim()}
            onClick={() =>
              void open(() =>
                createMap(campaignId, {
                  name: name.trim(),
                  copy: copy || undefined,
                }),
              )
            }
          >
            {t('maps.create')}
          </Btn>
        </Panel>

        <Panel title={t('maps.importTitle')}>
          <label className="flex flex-col gap-1 text-caption">
            {t('maps.file')}
            <input
              type="file"
              accept=".dd2vtt,.uvtt,.df2vtt,.json,image/png,image/jpeg,image/webp"
              disabled={busy}
              onChange={(e) => {
                const file = e.target.files?.[0]
                if (file) void chooseFile(file)
                e.target.value = ''
              }}
            />
          </label>
          <p className="text-caption text-mute-soft">{t('maps.importHint')}</p>
        </Panel>
      </div>

      {aligning && (
        <AlignPanel
          aligning={aligning}
          busy={busy}
          onCancel={() => setAligning(null)}
          onImport={(body) => void open(() => importMap(campaignId, body))}
        />
      )}
    </main>
  )
}

/** The GM lays a grid on an image: cell size and where the first whole cell starts. */
function AlignPanel({
  aligning,
  busy,
  onCancel,
  onImport,
}: {
  aligning: Aligning
  busy: boolean
  onCancel: () => void
  onImport: (body: MapImport) => void
}) {
  const { t } = useTranslation()
  // What the GM types, kept as typed: clamping each keystroke would turn « 64 » into « 864 ».
  const [typed, setTyped] = useState({
    cell: String(Math.max(8, Math.round(aligning.width / 20))),
    ox: '0',
    oy: '0',
  })
  const cell = Math.max(8, Math.floor(Number(typed.cell)) || 8)
  const ox = Math.max(0, Math.floor(Number(typed.ox)) || 0)
  const oy = Math.max(0, Math.floor(Number(typed.oy)) || 0)
  const canvas = useRef<HTMLCanvasElement>(null)
  const columns = Math.floor((aligning.width - ox) / cell)
  const rows = Math.floor((aligning.height - oy) / cell)
  const fits = columns >= 3 && rows >= 3 && columns <= 64 && rows <= 64

  useEffect(() => {
    const ctx = canvas.current?.getContext('2d')
    if (!ctx) return
    const img = new Image()
    img.onload = () => {
      const c = ctx.canvas
      ctx.drawImage(img, 0, 0, c.width, c.height)
      const k = c.width / aligning.width
      ctx.strokeStyle = 'rgba(224,166,80,0.9)'
      ctx.lineWidth = 1
      for (let i = 0; i <= columns; i++) {
        const x = (ox + i * cell) * k
        ctx.beginPath()
        ctx.moveTo(x, oy * k)
        ctx.lineTo(x, (oy + rows * cell) * k)
        ctx.stroke()
      }
      for (let j = 0; j <= rows; j++) {
        const y = (oy + j * cell) * k
        ctx.beginPath()
        ctx.moveTo(ox * k, y)
        ctx.lineTo((ox + columns * cell) * k, y)
        ctx.stroke()
      }
    }
    img.src = aligning.dataUrl
  }, [aligning, cell, ox, oy, columns, rows])

  const width = Math.min(720, aligning.width)

  return (
    <Panel title={t('maps.align.title', { name: aligning.name })}>
      <div className="flex flex-wrap gap-3">
        {ALIGN_FIELDS.map(([key, label, min]) => (
          <label key={key} className="flex flex-col gap-1 text-caption">
            {t(label)}
            <input
              type="number"
              className={field}
              min={min}
              value={typed[key]}
              onChange={(e) => setTyped((v) => ({ ...v, [key]: e.target.value }))}
            />
          </label>
        ))}
      </div>
      <p className="text-caption text-mute-soft">{t('maps.align.size', { columns, rows })}</p>
      <canvas
        ref={canvas}
        width={width}
        height={Math.round((aligning.height * width) / aligning.width)}
        className="max-w-full rounded-md border border-line"
        aria-label={t('maps.align.preview')}
      />
      <div className="flex gap-2">
        <Btn
          main
          disabled={busy || !fits}
          onClick={() =>
            onImport({
              kind: 'image',
              name: aligning.name,
              image: aligning.dataUrl,
              cellPx: cell,
              offsetX: ox,
              offsetY: oy,
              columns,
              rows,
            })
          }
        >
          {t('maps.align.import')}
        </Btn>
        <Btn onClick={onCancel}>{t('maps.align.cancel')}</Btn>
      </div>
      {!fits && <p className="text-caption text-stat-atk">{t('maps.align.tooSmall')}</p>}
    </Panel>
  )
}
