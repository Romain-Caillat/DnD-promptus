import * as THREE from 'three'
import type { Figure, Paint } from './build'

/**
 * The canvases the HD-2D scene is painted with: a PixelLab image when
 * the theme has one, else a 16-pixel texture drawn from the theme's
 * colours (the same patterns as the flat renderer), and the marks of
 * the Atlas as flat pieces. Pixel art stays pixel art: nearest filter.
 */

function rng(seed: number) {
  let s = seed | 0
  return () => {
    s = (s + 0x6d2b79f5) | 0
    let t = Math.imul(s ^ (s >>> 15), 1 | s)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

function canvas(w: number, h: number): [HTMLCanvasElement, CanvasRenderingContext2D] {
  const c = document.createElement('canvas')
  c.width = w
  c.height = h
  const g = c.getContext('2d')
  if (!g) throw new Error('no 2d context')
  return [c, g]
}

function pixelTexture(source: HTMLCanvasElement | HTMLImageElement): THREE.Texture {
  const t = source instanceof HTMLCanvasElement ? new THREE.CanvasTexture(source) : new THREE.Texture(source)
  t.magFilter = THREE.NearestFilter
  t.minFilter = THREE.NearestFilter
  t.generateMipmaps = false
  t.wrapS = THREE.RepeatWrapping
  t.wrapT = THREE.RepeatWrapping
  t.colorSpace = THREE.SRGBColorSpace
  t.needsUpdate = true
  return t
}

/** A 16 × 16 tile of the theme's pattern in its three tones. */
function themeCanvas(p: Extract<Paint, { kind: 'theme' }>): HTMLCanvasElement {
  const [c, g] = canvas(16, 16)
  const r = rng(p.base.length * 31 + p.pattern.length)
  g.fillStyle = p.base
  g.fillRect(0, 0, 16, 16)
  const px = (x: number, y: number, w: number, h: number, col: string) => {
    g.fillStyle = col
    g.fillRect(x, y, w, h)
  }
  switch (p.pattern) {
    case 'cobbles':
    case 'flagstones':
    case 'stones':
    case 'bricks': {
      const bw = p.pattern === 'cobbles' ? 4 : 8
      const bh = p.pattern === 'cobbles' ? 3 : 4
      px(0, 0, 16, 16, p.dark)
      for (let y = 0; y < 16; y += bh + 1)
        for (let x = ((y / (bh + 1)) % 2) * (bw / 2) - bw; x < 16; x += bw + 1) {
          px(x, y, bw, bh, r() < 0.3 ? p.light : p.base)
          px(x, y, bw, 1, p.light)
        }
      break
    }
    case 'planks':
    case 'grating':
      for (let y = 0; y < 16; y += 4) {
        px(0, y + 3, 16, 1, p.dark)
        px(Math.floor(r() * 16), y, 1, 3, p.dark)
        px(0, y, 16, 1, p.light)
      }
      break
    case 'water':
      for (let i = 0; i < 14; i++) px(Math.floor(r() * 16), Math.floor(r() * 16), 2 + Math.floor(r() * 4), 1, i % 3 ? p.light : p.dark)
      break
    case 'plates':
      px(0, 0, 16, 1, p.light)
      px(0, 8, 16, 1, p.dark)
      px(8, 0, 1, 16, p.dark)
      px(2, 2, 1, 1, p.dark)
      px(13, 13, 1, 1, p.dark)
      break
    default:
      for (let i = 0; i < 24; i++) px(Math.floor(r() * 16), Math.floor(r() * 16), 1, 1, r() < 0.5 ? p.dark : p.light)
  }
  return c
}

const images = new Map<string, HTMLImageElement>()

/**
 * A texture for `paint`; an image loads in the background and
 * `onLoad` asks the scene to draw again.
 */
export function textureOf(paint: Paint, onLoad: () => void): THREE.Texture {
  if (paint.kind === 'theme') return pixelTexture(themeCanvas(paint))
  let img = images.get(paint.url)
  if (!img) {
    img = new Image()
    img.crossOrigin = 'use-credentials'
    img.src = paint.url
    images.set(paint.url, img)
  }
  const t = pixelTexture(img)
  if (img.complete && img.naturalWidth > 0) {
    t.needsUpdate = true
  } else {
    img.addEventListener(
      'load',
      () => {
        t.needsUpdate = true
        onLoad()
      },
      { once: true },
    )
  }
  return t
}

/** An image texture for a figure's PixelLab picture. */
export function imageTexture(url: string, onLoad: () => void): THREE.Texture {
  return textureOf({ kind: 'image', url, pending: false, subject: url }, onLoad)
}

/**
 * A mark of the Atlas as a flat piece (64 × 96): an NPC a round piece
 * holding its initial, a clue a diamond, a fight a crossed square, a
 * scene's pin a flag with its numbers. Claude's are dotted and marked ▶.
 */
export function markCanvas(f: Figure): HTMLCanvasElement {
  const [c, g] = canvas(64, 96)
  const claude = f.proposal !== null
  const ink = claude ? '#e0a650' : '#efe6d2'
  g.globalAlpha = f.faint ? 0.55 : 1
  g.lineWidth = 4
  g.strokeStyle = f.picked ? '#ffd60a' : ink
  if (claude) g.setLineDash([6, 5])
  g.fillStyle = '#14100c'
  g.beginPath()
  switch (f.shape) {
    case 'npc':
      g.arc(32, 40, 24, 0, Math.PI * 2)
      break
    case 'clue':
      g.moveTo(32, 12)
      g.lineTo(56, 40)
      g.lineTo(32, 68)
      g.lineTo(8, 40)
      g.closePath()
      break
    case 'fight':
      g.rect(10, 18, 44, 44)
      break
    default:
      g.moveTo(14, 70)
      g.lineTo(14, 14)
      g.lineTo(54, 14)
      g.lineTo(54, 50)
      g.lineTo(18, 50)
  }
  g.fill()
  g.stroke()
  g.setLineDash([])
  g.fillStyle = ink
  g.font = 'bold 22px monospace'
  g.textAlign = 'center'
  g.textBaseline = 'middle'
  if (f.shape === 'fight') {
    g.beginPath()
    g.moveTo(16, 24)
    g.lineTo(48, 56)
    g.moveTo(48, 24)
    g.lineTo(16, 56)
    g.stroke()
  } else if (f.label) {
    g.fillText(f.label.slice(0, 5), f.shape === 'pin' ? 34 : 32, f.shape === 'pin' ? 32 : 41)
  }
  if (claude) {
    // ▶ drawn in cells: some fonts have no triangle.
    g.fillStyle = '#e0a650'
    g.beginPath()
    g.moveTo(4, 78)
    g.lineTo(18, 86)
    g.lineTo(4, 94)
    g.fill()
  }
  return c
}

export function canvasTexture(c: HTMLCanvasElement): THREE.Texture {
  const t = pixelTexture(c)
  t.wrapS = THREE.ClampToEdgeWrapping
  t.wrapT = THREE.ClampToEdgeWrapping
  return t
}

/** A thin prop without its PixelLab image: a post in its colour. */
export function flatCanvas(color: string): HTMLCanvasElement {
  const [c, g] = canvas(16, 32)
  g.fillStyle = color
  g.fillRect(6, 4, 4, 28)
  g.fillRect(3, 4, 10, 3)
  g.fillStyle = 'rgba(0,0,0,0.35)'
  g.fillRect(9, 4, 1, 28)
  return c
}
