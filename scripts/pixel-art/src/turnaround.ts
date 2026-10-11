import { createBitmap, type Bitmap } from './image'
import { BASELINE, GABARITS, type Gabarit, type View } from './pose'

export const TURNAROUND_VIEWS: readonly View[] = ['front', 'side', 'back']
const FRAME = 512

/**
 * Cuts a turnaround sheet into its views. Only the box where the
 * mannequin stands is kept: the model likes to fill the empty space
 * above short figures with extra thumbnails, and those must not leak in.
 */
export function splitTurnaround(sheet: Bitmap, gabarit: Gabarit): Record<View, Bitmap> {
  const views = {} as Record<View, Bitmap>
  TURNAROUND_VIEWS.forEach((view, column) => {
    views[view] = cropColumn(sheet, gabarit, column)
  })
  return views
}

/**
 * Keeps the mannequin's box in one 512 px column. `widen` makes room
 * for arms flung out in a key pose; the box never crosses into the
 * neighbouring column.
 */
export function cropColumn(sheet: Bitmap, gabarit: Gabarit, column: number, widen = 1): Bitmap {
  const { height, width } = GABARITS[gabarit]
  const top = Math.max(0, Math.round(BASELINE - height * 1.08))
  const half = Math.round(height * width * 0.42 * widen)
  const x0 = Math.max(column * FRAME, column * FRAME + FRAME / 2 - half)
  const x1 = Math.min(sheet.width, (column + 1) * FRAME, column * FRAME + FRAME / 2 + half)
  const out = createBitmap(x1 - x0, sheet.height - top)
  for (let y = 0; y < out.height; y++) {
    const from = ((top + y) * sheet.width + x0) * 4
    out.data.set(sheet.data.subarray(from, from + out.width * 4), y * out.width * 4)
  }
  return out
}
