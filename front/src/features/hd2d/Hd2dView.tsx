import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import * as THREE from 'three'
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'
import { cn } from '@/lib/utils'
import type { Hd2dScene, Paint } from './build'
import { canvasTexture, flatCanvas, imageTexture, markCanvas, textureOf } from './paint'

/** The light of each hour: sky, ground, sun, and the fog's colour. */
const HOURS = {
  dawn: { sky: '#c9a0a8', ground: '#2a2228', sun: '#ffc9a0', sunI: 0.9, fog: '#5a4a58' },
  day: { sky: '#cfe0f0', ground: '#4a4030', sun: '#fff4e0', sunI: 1.6, fog: '#a9b8c4' },
  dusk: { sky: '#d08860', ground: '#2a1c18', sun: '#ff9a60', sunI: 0.8, fog: '#4a3038' },
  night: { sky: '#6a84a8', ground: '#2c241e', sun: '#a8bfe8', sunI: 1.2, fog: '#1a2633' },
} as const

const WALL_HEIGHT = 1.6
const VOLUME_HEIGHT = 0.8

/** Whether this device can draw WebGL at all; the probe's context is let go at once. */
function webgl(): boolean {
  try {
    const c = document.createElement('canvas')
    const gl = c.getContext('webgl2') ?? c.getContext('webgl')
    gl?.getExtension('WEBGL_lose_context')?.loseContext()
    return Boolean(gl)
  } catch {
    return false
  }
}

const reduced = () => typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches

/** What one scene put in the stage: what the loop animates and taps hit, and how to let it go. */
interface Content {
  three: THREE.Scene
  pickable: THREE.Sprite[]
  pulsing: THREE.Material[]
  lamps: { light: THREE.PointLight; flicker: boolean; seed: number }[]
  dispose: () => void
}

/** The scene's objects, lights and fog, from its description. */
function populate(scene: Hd2dScene, redraw: () => void): Content {
  const hour = HOURS[scene.time]
  const three = new THREE.Scene()
  three.background = new THREE.Color(hour.fog)
  const span = Math.max(scene.width, scene.height, 4)
  // The map's middle stays readable: half the light there in fog, most of it without.
  three.fog = new THREE.FogExp2(hour.fog, (scene.fog ? 0.55 : 0.3) / (span * 0.95))
  const cx = scene.width / 2
  const cz = scene.height / 2
  three.add(new THREE.HemisphereLight(hour.sky, hour.ground, scene.time === 'night' ? 1.6 : 1.8))
  const sun = new THREE.DirectionalLight(hour.sun, hour.sunI)
  sun.position.set(cx - span * 0.4, span, cz + span * 0.5)
  sun.target.position.set(cx, 0, cz)
  sun.castShadow = true
  sun.shadow.mapSize.set(2048, 2048)
  Object.assign(sun.shadow.camera, { left: -span, right: span, top: span, bottom: -span, near: 0.5, far: span * 4 })
  sun.shadow.bias = -0.0008
  three.add(sun, sun.target)

  const disposables: { dispose(): void }[] = []
  const keep = <T extends { dispose(): void }>(x: T): T => {
    disposables.push(x)
    return x
  }
  const pulsing: THREE.Material[] = []
  const material = (paint: Paint, repeat = 1) => {
    const tex = keep(textureOf(paint, redraw))
    tex.repeat.set(repeat, repeat)
    const m = keep(new THREE.MeshStandardMaterial({ map: tex, roughness: 0.85, metalness: 0 }))
    if (paint.kind === 'image' && paint.pending) {
      m.emissive = new THREE.Color('#e0a650')
      pulsing.push(m)
    }
    return m
  }
  const at = (x: number, y: number, lift = 0) => new THREE.Vector3(x + 0.5, lift, y + 0.5)
  const matrix = new THREE.Matrix4()

  // Floors: one instanced plane per material, water a little lower.
  const plane = keep(new THREE.PlaneGeometry(1, 1).rotateX(-Math.PI / 2))
  for (const f of scene.floors) {
    const m = material(f.paint)
    if (f.water !== 'none') {
      m.transparent = true
      m.opacity = f.water === 'deep' ? 0.92 : 0.8
      m.roughness = 0.1
      m.metalness = 0.3
    }
    const mesh = new THREE.InstancedMesh(plane, m, f.cells.length)
    f.cells.forEach(([x, y], i) => {
      const lift = (f.heights[i] ?? 0) * 0.5 - (f.water === 'deep' ? 0.25 : f.water === 'shallow' ? 0.08 : 0)
      mesh.setMatrixAt(i, matrix.makeTranslation(at(x, y, lift)))
    })
    mesh.receiveShadow = true
    three.add(mesh)
  }

  // Walls: textured blocks.
  if (scene.walls.length > 0) {
    const box = keep(new THREE.BoxGeometry(1, WALL_HEIGHT, 1))
    const mesh = new THREE.InstancedMesh(box, material(scene.wallPaint), scene.walls.length)
    scene.walls.forEach(([x, y], i) => mesh.setMatrixAt(i, matrix.makeTranslation(at(x, y, WALL_HEIGHT / 2))))
    mesh.castShadow = true
    mesh.receiveShadow = true
    three.add(mesh)
  }

  // Props with volume: a box, its faces from PixelLab or the theme.
  for (const v of scene.volumes) {
    const [w, d] = v.size
    const geo = keep(new THREE.BoxGeometry(w * 0.9, VOLUME_HEIGHT, d * 0.9))
    const side = material(v.side)
    const top = material(v.top)
    const mesh = new THREE.Mesh(geo, [side, side, top, side, side, side])
    mesh.position.set(v.at[0] + w / 2, VOLUME_HEIGHT / 2, v.at[1] + d / 2)
    mesh.castShadow = true
    mesh.receiveShadow = true
    mesh.name = v.name
    three.add(mesh)
  }

  // Thin props, then the marks: flat pieces turned to the camera.
  const pickable: THREE.Sprite[] = []
  const sprite = (tex: THREE.Texture, x: number, y: number, h: number, ratio: number, opacity = 1) => {
    const m = keep(new THREE.SpriteMaterial({ map: tex, transparent: true, opacity, depthWrite: false }))
    const s = new THREE.Sprite(m)
    s.scale.set(h * ratio, h, 1)
    s.position.copy(at(x, y, h / 2))
    s.center.set(0.5, 0.5)
    three.add(s)
    return s
  }
  for (const f of scene.flats) {
    const tex = keep(f.paint ? textureOf(f.paint, redraw) : canvasTexture(flatCanvas(f.color)))
    sprite(tex, f.at[0], f.at[1], 1.2, 0.5)
  }
  for (const f of scene.figures) {
    const claude = f.proposal !== null
    const tex = keep(f.image ? imageTexture(f.image.url, redraw) : canvasTexture(markCanvas(f)))
    const s = sprite(tex, f.at[0], f.at[1], f.image ? 1.6 : 1.5, f.image ? 1 : 2 / 3, claude ? 0.75 : f.faint ? 0.6 : 1)
    s.userData.key = f.key
    if (claude) s.userData.claude = true
    if (f.image && claude) {
      // A figure Claude proposes keeps its ▶ above it.
      const mark = sprite(keep(canvasTexture(markCanvas({ ...f, image: null, label: '' }))), f.at[0], f.at[1], 0.5, 2 / 3, 0.9)
      mark.position.y = 1.7
    }
    pickable.push(s)
  }

  // Claude's map operations: the cells they touch glow.
  if (scene.proposedCells.length > 0) {
    const glow = keep(new THREE.MeshBasicMaterial({ color: '#e0a650', transparent: true, opacity: 0.35, depthWrite: false }))
    pulsing.push(glow)
    const mesh = new THREE.InstancedMesh(plane, glow, scene.proposedCells.length)
    scene.proposedCells.forEach(([x, y], i) => mesh.setMatrixAt(i, matrix.makeTranslation(at(x, y, 0.03))))
    three.add(mesh)
  }

  // The map's lights, the eight first (a phone's budget).
  const lamps = scene.lights.slice(0, 8).map((l) => {
    const p = new THREE.PointLight(l.color, 12, Math.max(3, l.bright * 3), 1.2)
    p.position.copy(at(l.at[0], l.at[1], 1.2))
    three.add(p)
    return { light: p, flicker: l.flicker, seed: l.at[0] * 7 + l.at[1] * 13 }
  })

  return {
    three,
    pickable,
    pulsing,
    lamps,
    dispose: () => {
      for (const d of disposables) d.dispose()
      sun.shadow.dispose()
    },
  }
}

/**
 * campaign/watch-prep-in-hd2d — the scene being prepared, drawn in
 * HD-2D (`MEMORY.md` §2): a Three.js scene built from the grid, lit by
 * the map's hour and lights, with its fog; walls and props with volume
 * are textured boxes, thin props and every mark of the Atlas are flat
 * pieces turned to the camera. What Claude proposes floats translucent,
 * dotted and marked ▶, and its cells glow; a PixelLab element still
 * waiting for the GM pulses. A tap on a mark answers its key. Drag to
 * turn, pinch or wheel to come closer.
 *
 * One renderer (one WebGL context) lives as long as the view: a new
 * revision only swaps the scene's contents, and the GM's camera stays.
 *
 * Without WebGL (an old device, the tests) it says so: the flat map
 * stays one tap away.
 */
export function Hd2dView({
  scene: input,
  onMark,
  className,
}: {
  scene: Hd2dScene
  onMark?: (key: string) => void
  className?: string
}) {
  const { t } = useTranslation()
  const host = useRef<HTMLDivElement>(null)
  const [able] = useState(webgl)
  const onMarkRef = useRef(onMark)
  useEffect(() => {
    onMarkRef.current = onMark
  }, [onMark])
  /** The mounted stage: set by the first effect, filled by the second. */
  const stage = useRef<{
    show: (scene: Hd2dScene) => void
  } | null>(null)
  // The page hands the same object while nothing changed (`buildSceneOnce`).
  const signature = useMemo(() => JSON.stringify(input), [input])

  useEffect(() => {
    const el = host.current
    if (!able || !el) return
    let renderer: THREE.WebGLRenderer
    try {
      renderer = new THREE.WebGLRenderer({ antialias: false, powerPreference: 'high-performance' })
    } catch {
      return
    }
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    renderer.outputColorSpace = THREE.SRGBColorSpace
    renderer.toneMapping = THREE.ACESFilmicToneMapping
    renderer.toneMappingExposure = 1.5
    renderer.shadowMap.enabled = true
    renderer.shadowMap.type = THREE.PCFSoftShadowMap
    el.appendChild(renderer.domElement)

    const camera = new THREE.PerspectiveCamera(38, 1, 0.1, 400)
    const controls = new OrbitControls(camera, renderer.domElement)
    controls.enableDamping = true
    controls.minPolarAngle = 0.35
    controls.maxPolarAngle = 1.2
    controls.minDistance = 3

    let dirty = true
    const redraw = () => {
      dirty = true
    }
    let content: Content | null = null
    let framed = ''
    stage.current = {
      show: (scene) => {
        const next = populate(scene, redraw)
        content?.dispose()
        content = next
        // The camera is placed once per map size; afterwards the GM's view stays.
        const size = `${scene.width}x${scene.height}`
        if (framed !== size) {
          framed = size
          const span = Math.max(scene.width, scene.height, 4)
          const cx = scene.width / 2
          const cz = scene.height / 2
          controls.target.set(cx, 0, cz)
          camera.position.set(cx, span * 0.6, cz + span * 0.7)
          controls.maxDistance = span * 2.5
          controls.update()
        }
        redraw()
      },
    }

    const resize = () => {
      const w = el.clientWidth || 640
      const h = el.clientHeight || 420
      renderer.setSize(w, h, false)
      camera.aspect = w / h
      camera.updateProjectionMatrix()
      redraw()
    }
    resize()
    const observer = typeof ResizeObserver === 'function' ? new ResizeObserver(resize) : null
    observer?.observe(el)
    controls.addEventListener('change', redraw)

    // A tap (not a drag) on a mark answers its key.
    const ray = new THREE.Raycaster()
    let down: { x: number; y: number } | null = null
    const onDown = (e: PointerEvent) => {
      down = { x: e.clientX, y: e.clientY }
    }
    const onUp = (e: PointerEvent) => {
      if (!content || !down || Math.hypot(e.clientX - down.x, e.clientY - down.y) > 6) return
      const r = renderer.domElement.getBoundingClientRect()
      ray.setFromCamera(new THREE.Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1), camera)
      const hit = ray.intersectObjects(content.pickable)[0]
      const key = hit?.object.userData.key as string | undefined
      if (key) onMarkRef.current?.(key)
    }
    renderer.domElement.addEventListener('pointerdown', onDown)
    renderer.domElement.addEventListener('pointerup', onUp)

    const still = reduced()
    let frame = 0
    const clock = new THREE.Clock()
    const tick = () => {
      frame = requestAnimationFrame(tick)
      if (!content) return
      const time = clock.getElapsedTime()
      controls.update()
      const animate = !still && (content.pulsing.length > 0 || content.lamps.some((l) => l.flicker))
      if (!animate && !dirty) return
      dirty = false
      if (!still) {
        const pulse = 0.25 + 0.25 * Math.sin(time * 3)
        for (const m of content.pulsing) {
          if (m instanceof THREE.MeshStandardMaterial) m.emissiveIntensity = pulse
          else m.opacity = 0.2 + pulse * 0.6
        }
        for (const l of content.lamps)
          if (l.flicker) l.light.intensity = 12 + Math.sin(time * 9 + l.seed) * 1.5 + Math.sin(time * 23 + l.seed) * 0.9
      }
      renderer.render(content.three, camera)
    }
    tick()

    return () => {
      stage.current = null
      cancelAnimationFrame(frame)
      observer?.disconnect()
      renderer.domElement.removeEventListener('pointerdown', onDown)
      renderer.domElement.removeEventListener('pointerup', onUp)
      controls.dispose()
      content?.dispose()
      renderer.dispose()
      // Give the WebGL context back now, not when the collector gets to it.
      renderer.forceContextLoss()
      renderer.domElement.remove()
    }
  }, [able])

  useEffect(() => {
    stage.current?.show(JSON.parse(signature) as Hd2dScene)
  }, [able, signature])

  if (!able) {
    return (
      <div className={cn('surface-slab flex min-h-60 items-center justify-center p-6 text-center', className)}>
        <p role="status" className="text-body text-mute-soft">
          {t('atlas.hd2d.unavailable')}
        </p>
      </div>
    )
  }
  return (
    <div
      ref={host}
      role="img"
      aria-label={t('atlas.hd2d.label')}
      className={cn('relative aspect-[16/10] w-full touch-none overflow-hidden rounded-panel bg-black', className)}
    />
  )
}
