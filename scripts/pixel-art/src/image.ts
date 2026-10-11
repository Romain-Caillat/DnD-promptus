import { PNG } from 'pngjs'

/** An RGBA bitmap, 4 bytes per pixel, rows top to bottom. */
export interface Bitmap {
  width: number
  height: number
  data: Uint8Array
}

export type Rgb = readonly [number, number, number]

export function createBitmap(width: number, height: number): Bitmap {
  return { width, height, data: new Uint8Array(width * height * 4) }
}

export async function readPng(path: string): Promise<Bitmap> {
  const png = PNG.sync.read(Buffer.from(await Bun.file(path).arrayBuffer()))
  return { width: png.width, height: png.height, data: new Uint8Array(png.data) }
}

export async function writePng(path: string, bitmap: Bitmap): Promise<void> {
  const png = new PNG({ width: bitmap.width, height: bitmap.height })
  png.data = Buffer.from(bitmap.data)
  await Bun.write(path, PNG.sync.write(png))
}

export function rgbAt(bitmap: Bitmap, x: number, y: number): Rgb {
  const i = (y * bitmap.width + x) * 4
  const d = bitmap.data
  return [d[i]!, d[i + 1]!, d[i + 2]!]
}

export function alphaAt(bitmap: Bitmap, x: number, y: number): number {
  return bitmap.data[(y * bitmap.width + x) * 4 + 3]!
}

export function setPixel(bitmap: Bitmap, x: number, y: number, rgb: Rgb, alpha = 255): void {
  const i = (y * bitmap.width + x) * 4
  bitmap.data[i] = rgb[0]
  bitmap.data[i + 1] = rgb[1]
  bitmap.data[i + 2] = rgb[2]
  bitmap.data[i + 3] = alpha
}

/** Nearest-neighbour upscale, for previews: pixel art must stay crisp. */
export function scaleUp(bitmap: Bitmap, factor: number): Bitmap {
  const out = createBitmap(bitmap.width * factor, bitmap.height * factor)
  for (let y = 0; y < out.height; y++) {
    for (let x = 0; x < out.width; x++) {
      const src = (Math.floor(y / factor) * bitmap.width + Math.floor(x / factor)) * 4
      out.data.set(bitmap.data.subarray(src, src + 4), (y * out.width + x) * 4)
    }
  }
  return out
}

/** Copies `src` into `dst` at (dx, dy), alpha-blending over what is there. */
export function blit(dst: Bitmap, src: Bitmap, dx: number, dy: number): void {
  for (let y = 0; y < src.height; y++) {
    for (let x = 0; x < src.width; x++) {
      const tx = dx + x
      const ty = dy + y
      if (tx < 0 || ty < 0 || tx >= dst.width || ty >= dst.height) continue
      const a = alphaAt(src, x, y)
      if (a === 0) continue
      const [r, g, b] = rgbAt(src, x, y)
      const [br, bg, bb] = rgbAt(dst, tx, ty)
      const t = a / 255
      setPixel(dst, tx, ty, [
        Math.round(r * t + br * (1 - t)),
        Math.round(g * t + bg * (1 - t)),
        Math.round(b * t + bb * (1 - t)),
      ])
    }
  }
}
