import { createBitmap, setPixel, type Bitmap, type Rgb } from './image'

/**
 * The mannequin: one stocky, big-headed body in three views, drawn the
 * way OpenPose draws a skeleton, which is what ControlNet's pose mode
 * was trained on. Every character is generated onto it, so they all
 * share stance and proportions; only the size class changes.
 *
 * Keypoints follow the OpenPose 18-point order, in units of body
 * height: y = 0 at the top of the head, y = 1 under the feet, x = 0 on
 * the body's axis.
 */
type Point = readonly [number, number] | null

/** nose, neck, R shoulder/elbow/wrist, L shoulder/elbow/wrist, R hip/knee/ankle, L hip/knee/ankle, R eye, L eye, R ear, L ear */
export type Skeleton = readonly Point[]

export type View = 'front' | 'side' | 'back'

/** Facing the viewer: the character's right side is on the image's left. */
const FRONT: Skeleton = [
  [0, 0.215], [0, 0.35], [-0.15, 0.37], [-0.185, 0.5], [-0.2, 0.615], [0.15, 0.37], [0.185, 0.5], [0.2, 0.615],
  [-0.075, 0.66], [-0.085, 0.82], [-0.095, 0.97], [0.075, 0.66], [0.085, 0.82], [0.095, 0.97],
  [-0.05, 0.175], [0.05, 0.175], [-0.11, 0.19], [0.11, 0.19],
]

/** Seen from behind: sides swap, the face disappears, the ears remain. */
const BACK: Skeleton = FRONT.map((p, i) => (p === null || i === 0 || i === 14 || i === 15 ? null : [-p[0], p[1]]))

/** Profile facing right: the left side faces the viewer, the right one hides behind. */
const SIDE: Skeleton = [
  [0.095, 0.22], [0, 0.35], [-0.01, 0.37], [-0.035, 0.5], [-0.035, 0.61], [0.01, 0.37], [0.025, 0.5], [0.045, 0.62],
  [-0.01, 0.66], [-0.025, 0.82], [-0.035, 0.97], [0.01, 0.66], [0.025, 0.82], [0.025, 0.97],
  null, [0.06, 0.18], null, [-0.03, 0.19],
]

export const SKELETONS: Record<View, Skeleton> = { front: FRONT, side: SIDE, back: BACK }

/** Replaces some keypoints of a skeleton. */
function bend(skeleton: Skeleton, moves: Record<number, readonly [number, number]>): Skeleton {
  return skeleton.map((p, i) => moves[i] ?? p)
}

/**
 * Key poses, seen from the front, drawn by the model rather than moved
 * by the rig: a swing needs an arm that really rises. Hands stay empty;
 * the weapon is placed at the wrist afterwards.
 */
export const KEY_POSES = {
  /** Right arm raised overhead, feet apart. */
  windup: bend(FRONT, { 3: [-0.21, 0.23], 4: [-0.15, 0.07], 10: [-0.13, 0.97], 13: [0.12, 0.97] }),
  /** Right arm swung out to the side, after the blow. */
  strike: bend(FRONT, { 3: [-0.27, 0.43], 4: [-0.36, 0.52], 10: [-0.13, 0.97], 13: [0.12, 0.97] }),
  /** Both arms raised, casting. */
  cast: bend(FRONT, { 3: [-0.24, 0.27], 4: [-0.27, 0.13], 6: [0.24, 0.27], 7: [0.27, 0.13] }),
} as const

export type KeyPose = keyof typeof KEY_POSES

/**
 * Size classes, as a share of the 640 px generation frame (80 latent
 * cells, so 80 art pixels): small ≈ 36 px, medium ≈ 48 px, large ≈ 60 px
 * tall once normalized, all standing on the same baseline.
 */
export const GABARITS = {
  petit: { height: 288, width: 1.05 },
  moyen: { height: 384, width: 1.15 },
  grand: { height: 480, width: 1.35 },
} as const

export type Gabarit = keyof typeof GABARITS

/** Where the feet stand in a 640 px tall generation frame. */
export const BASELINE = 610

const LIMBS = [
  [1, 2], [1, 5], [2, 3], [3, 4], [5, 6], [6, 7], [1, 8], [8, 9], [9, 10],
  [1, 11], [11, 12], [12, 13], [1, 0], [0, 14], [14, 16], [0, 15], [15, 17],
] as const

const COLOURS: readonly Rgb[] = [
  [255, 0, 0], [255, 85, 0], [255, 170, 0], [255, 255, 0], [170, 255, 0], [85, 255, 0],
  [0, 255, 0], [0, 255, 85], [0, 255, 170], [0, 255, 255], [0, 170, 255], [0, 85, 255],
  [0, 0, 255], [85, 0, 255], [170, 0, 255], [255, 0, 255], [255, 0, 170], [255, 0, 85],
]

/** Keypoint in frame pixels, for a column centred on `centreX`. */
export function placePoint(p: readonly [number, number], gabarit: Gabarit, centreX: number): [number, number] {
  const { height, width } = GABARITS[gabarit]
  return [centreX + p[0] * height * width, BASELINE - height + p[1] * height]
}

/** Draws the mannequin in each view, one `frame`-wide column per view, 640 px tall. */
export function drawPoses(views: readonly View[], gabarit: Gabarit, frame = 512): Bitmap {
  return drawSkeletons(views.map((v) => SKELETONS[v]), gabarit, frame)
}

/** Draws skeletons side by side, one `frame`-wide column each, 640 px tall. */
export function drawSkeletons(skeletons: readonly Skeleton[], gabarit: Gabarit, frame = 512): Bitmap {
  const out = createBitmap(frame * skeletons.length, 640)
  for (let i = 3; i < out.data.length; i += 4) out.data[i] = 255
  const stick = Math.max(3, GABARITS[gabarit].height / 70)
  skeletons.forEach((skeleton, column) => {
    const at = (p: readonly [number, number]) => placePoint(p, gabarit, column * frame + frame / 2)
    LIMBS.forEach(([a, b], i) => {
      const pa = skeleton[a]
      const pb = skeleton[b]
      if (!pa || !pb) return
      const colour = COLOURS[i]!.map((c) => Math.round(c * 0.6)) as unknown as Rgb
      line(out, at(pa), at(pb), stick, colour)
    })
    skeleton.forEach((p, i) => {
      if (p) disc(out, at(p), stick, COLOURS[i]!)
    })
  })
  return out
}

function line(out: Bitmap, a: readonly [number, number], b: readonly [number, number], radius: number, colour: Rgb) {
  const steps = Math.ceil(Math.hypot(b[0] - a[0], b[1] - a[1]))
  for (let s = 0; s <= steps; s++) {
    const t = steps === 0 ? 0 : s / steps
    disc(out, [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t], radius, colour)
  }
}

function disc(out: Bitmap, [cx, cy]: readonly [number, number], radius: number, colour: Rgb) {
  for (let y = Math.floor(cy - radius); y <= Math.ceil(cy + radius); y++) {
    for (let x = Math.floor(cx - radius); x <= Math.ceil(cx + radius); x++) {
      if (x < 0 || y < 0 || x >= out.width || y >= out.height) continue
      if ((x - cx) ** 2 + (y - cy) ** 2 <= radius * radius) setPixel(out, x, y, colour)
    }
  }
}
