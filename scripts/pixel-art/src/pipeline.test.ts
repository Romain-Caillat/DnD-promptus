import { describe, expect, test } from 'bun:test'
import { ANIMATION_NAMES, FRAME, renderSheet } from './animate'
import { alphaAt, createBitmap, rgbAt, setPixel, type Bitmap, type Rgb } from './image'
import { normalize } from './normalize'
import { HOUSE_32 } from './palette'

/** A small sprite blown up 8× on white, the way SDXL draws pixel art, with some noise. */
function fakeGeneration(art: (x: number, y: number) => Rgb | null, w: number, h: number): Bitmap {
  const out = createBitmap(512, 512)
  for (let y = 0; y < 512; y++) for (let x = 0; x < 512; x++) setPixel(out, x, y, [250, 250, 250])
  const left = 256 - w * 4
  const top = 480 - h * 8
  for (let y = 0; y < h * 8; y++) {
    for (let x = 0; x < w * 8; x++) {
      const c = art(Math.floor(x / 8), Math.floor(y / 8))
      if (!c) continue
      const noise = ((x * 7 + y * 13) % 5) - 2
      setPixel(out, left + x, top + y, [c[0] + noise, c[1] + noise, c[2] + noise])
    }
  }
  return out
}

const SKIN = HOUSE_32[10]!
const CLOTH = HOUSE_32[26]!

describe('normalize', () => {
  test('recovers one pixel per 8 px block, on the palette, background gone', () => {
    const art = (x: number, y: number): Rgb | null => (x === 0 || x === 11 ? null : y < 6 ? SKIN : CLOTH)
    const { sprite, cell } = normalize(fakeGeneration(art, 12, 24), { palette: HOUSE_32 })
    expect(cell).toBe(8)
    expect([sprite.width, sprite.height]).toEqual([10, 24])
    expect(rgbAt(sprite, 5, 2)).toEqual(SKIN)
    expect(rgbAt(sprite, 5, 20)).toEqual(CLOTH)
  })
})

describe('animate', () => {
  test('every view yields one row per animation, figures standing inside the frame', () => {
    const body = (w: number, h: number) => {
      const b = createBitmap(w, h)
      for (let y = 0; y < h; y++) for (let x = 2; x < w - 2; x++) setPixel(b, x, y, y < h / 3 ? SKIN : CLOTH)
      return b
    }
    const views = { front: body(24, 48), side: body(14, 48), back: body(24, 48) }
    const { sheet } = renderSheet({ views }, 'front')
    const longest = Math.max(...ANIMATION_NAMES.map((a) => a.frames))
    expect(sheet.width).toBe(longest * FRAME)
    expect(sheet.height).toBe(ANIMATION_NAMES.length * FRAME)
    // First frame of the idle row: feet on the floor line, nothing on the frame's edge.
    expect(alphaAt(sheet, FRAME / 2, 73)).toBe(255)
    expect(alphaAt(sheet, FRAME / 2, 74)).toBe(0)
    expect(alphaAt(sheet, 0, 0)).toBe(0)
  })
})
