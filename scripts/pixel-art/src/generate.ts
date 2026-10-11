#!/usr/bin/env bun
/**
 * Generates raw pixel art on a ComfyUI instance (the GM's GPU tower)
 * through its HTTP API, with SDXL and the Pixel Art XL LoRA.
 *
 * A character comes as a turnaround sheet: front, right profile and
 * back side by side in one 1536×640 image, posed on the mannequin by
 * ControlNet. One image keeps the three views far more alike than
 * three separate generations. Hands stay empty: weapons are items.
 *
 *   COMFYUI_URL=http://192.168.1.105:8188 bun run generate \
 *     --gabarit moyen --name nain --seeds 1,2,3 "a stout dwarf with a red beard…"
 *   COMFYUI_URL=… bun run generate --item --name hache --seeds 1,2 "dwarven battle axe"
 */
import { join } from 'node:path'
import { mkdir } from 'node:fs/promises'
import { parseArgs } from 'node:util'
import { PNG } from 'pngjs'
import { drawPoses, GABARITS, type Gabarit } from './pose'

const CHECKPOINT = 'sd_xl_base_1.0.safetensors'
const LORA = 'pixel-art-xl.safetensors'
const CONTROLNET = 'controlnet-union-sdxl-promax.safetensors'

const CHARACTER_NEGATIVE =
  'white outline, sticker border, white rim, weapon, sword, axe, shield, staff, dagger, holding object, extra items, ' +
  'extra characters, text, letters, blurry, smooth shading, 3d render, photo, cropped'
const ITEM_NEGATIVE = 'hand, character, person, multiple items, text, blurry, 3d render, photo, cropped'

const { values, positionals } = parseArgs({
  args: Bun.argv.slice(2),
  allowPositionals: true,
  options: {
    name: { type: 'string' },
    gabarit: { type: 'string', default: 'moyen' },
    item: { type: 'boolean', default: false },
    seeds: { type: 'string', default: '1' },
    out: { type: 'string', default: 'raw' },
  },
})

const host = process.env.COMFYUI_URL
const subject = positionals.join(' ')
const gabarit = values.gabarit as Gabarit
if (!host || !values.name || !subject || !(gabarit in GABARITS)) {
  console.error('usage: COMFYUI_URL=… generate --name <name> [--gabarit petit|moyen|grand | --item] [--seeds 1,2] "<subject>"')
  process.exit(1)
}

await mkdir(values.out, { recursive: true })
const pose = values.item ? null : await uploadPose(host, gabarit)
for (const seed of values.seeds.split(',').map(Number)) {
  const started = performance.now()
  const workflow = pose ? turnaround(subject, pose, seed) : item(subject, seed)
  const path = join(values.out, `${values.name}_${seed}.png`)
  await Bun.write(path, await run(host, workflow))
  console.log(`${path} in ${((performance.now() - started) / 1000).toFixed(1)} s`)
}

type Workflow = Record<string, { class_type: string; inputs: Record<string, unknown> }>

function base(text: string, negative: string, width: number, height: number, seed: number): Workflow {
  return {
    checkpoint: { class_type: 'CheckpointLoaderSimple', inputs: { ckpt_name: CHECKPOINT } },
    lora: {
      class_type: 'LoraLoader',
      inputs: { model: ['checkpoint', 0], clip: ['checkpoint', 1], lora_name: LORA, strength_model: 1, strength_clip: 1 },
    },
    positive: { class_type: 'CLIPTextEncode', inputs: { clip: ['lora', 1], text } },
    negative: { class_type: 'CLIPTextEncode', inputs: { clip: ['lora', 1], text: negative } },
    latent: { class_type: 'EmptyLatentImage', inputs: { width, height, batch_size: 1 } },
    sampler: {
      class_type: 'KSampler',
      inputs: {
        model: ['lora', 0],
        positive: ['positive', 0],
        negative: ['negative', 0],
        latent_image: ['latent', 0],
        seed,
        steps: 28,
        cfg: 6.5,
        sampler_name: 'dpmpp_2m',
        scheduler: 'karras',
        denoise: 1,
      },
    },
    decode: { class_type: 'VAEDecode', inputs: { samples: ['sampler', 0], vae: ['checkpoint', 2] } },
    save: { class_type: 'SaveImage', inputs: { images: ['decode', 0], filename_prefix: 'promptus' } },
  }
}

function turnaround(subject: string, pose: string, seed: number): Workflow {
  // The weighted front view: without it the model sometimes draws a back
  // in the first column, long hair being the usual culprit.
  const text =
    'character turnaround sheet showing the same character three times, from left to right: ' +
    '(front view facing the viewer with a visible face:1.3), side view facing right, back view. ' +
    `${subject}, empty hands, arms relaxed, full body, plain white background, pixel art, 16-bit rpg sprite`
  const workflow = base(text, CHARACTER_NEGATIVE, 1536, 640, seed)
  workflow.pose = { class_type: 'LoadImage', inputs: { image: pose } }
  workflow.controlnet = { class_type: 'ControlNetLoader', inputs: { control_net_name: CONTROLNET } }
  workflow.union = { class_type: 'SetUnionControlNetType', inputs: { control_net: ['controlnet', 0], type: 'openpose' } }
  workflow.apply = {
    class_type: 'ControlNetApplyAdvanced',
    inputs: {
      positive: ['positive', 0],
      negative: ['negative', 0],
      control_net: ['union', 0],
      image: ['pose', 0],
      strength: 0.95,
      start_percent: 0,
      end_percent: 0.85,
      vae: ['checkpoint', 2],
    },
  }
  workflow.sampler!.inputs.positive = ['apply', 0]
  workflow.sampler!.inputs.negative = ['apply', 1]
  return workflow
}

function item(subject: string, seed: number): Workflow {
  const text = `pixel art, a single ${subject}, game item icon, upright, centered, plain white background, 16-bit rpg item`
  return base(text, ITEM_NEGATIVE, 768, 768, seed)
}

async function uploadPose(host: string, gabarit: Gabarit): Promise<string> {
  const bitmap = drawPoses(['front', 'side', 'back'], gabarit)
  const png = new PNG({ width: bitmap.width, height: bitmap.height })
  png.data = Buffer.from(bitmap.data)
  const name = `promptus_pose_${gabarit}.png`
  const form = new FormData()
  form.append('image', new Blob([PNG.sync.write(png)], { type: 'image/png' }), name)
  form.append('overwrite', 'true')
  const response = await fetch(`${host}/upload/image`, { method: 'POST', body: form })
  if (!response.ok) throw new Error(`ComfyUI refused the pose: ${await response.text()}`)
  return ((await response.json()) as { name: string }).name
}

async function run(host: string, workflow: Workflow): Promise<ArrayBuffer> {
  const queued = await fetch(`${host}/prompt`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ prompt: workflow }),
  })
  if (!queued.ok) throw new Error(`ComfyUI refused the workflow: ${await queued.text()}`)
  const { prompt_id: id } = (await queued.json()) as { prompt_id: string }

  const deadline = Date.now() + 5 * 60_000
  while (Date.now() < deadline) {
    const history = (await (await fetch(`${host}/history/${id}`)).json()) as Record<
      string,
      {
        status: { status_str: string; messages: unknown }
        outputs: { save?: { images: { filename: string; subfolder: string; type: string }[] } }
      }
    >
    const entry = history[id]
    if (entry?.status.status_str === 'error') throw new Error(`ComfyUI failed: ${JSON.stringify(entry.status.messages)}`)
    const image = entry?.outputs.save?.images[0]
    if (image) return (await fetch(`${host}/view?${new URLSearchParams(image)}`)).arrayBuffer()
    await Bun.sleep(1000)
  }
  throw new Error('ComfyUI took over 5 minutes')
}
