# Pixel art pipeline

Turns diffusion output into sprites that share one format: an 80×80
frame, three size classes (36 / 48 / 60 px), the 32-colour house
palette, four directions and seven animations written once for every
character. The design canvas shows the result on the page
« Fondations · Sprites ».

## The chain

1. **Mannequin** (`src/pose.ts`): an OpenPose skeleton per size class,
   front, right profile and back.
2. **Generation** (`src/generate.ts`): SDXL + the Pixel Art XL LoRA on
   ComfyUI, posed by ControlNet union. The three views come in one
   1536×640 image, hands empty; weapons are generated as items.
3. **Normalization** (`src/normalize.ts`): background, shadow and specks
   removed, one pixel per 8 px block (SDXL's latent cell), colours
   snapped to `HOUSE_32`.
4. **Animation** (`src/animate.ts`): the sprite is sliced by the
   mannequin's proportions and the parts move by whole pixels; the
   weapon is held at the hand and rotates around its grip.
5. **Key poses** (`KEY_POSES` in `src/pose.ts`): a slice cannot raise
   an arm overhead, so the swing and the cast are drawn by the model,
   front view only for now. The rest view goes in the left panel, the
   right one is masked and inpainted, posed by ControlNet and held to
   the character by IP-Adapter. One inpainted panel per pass: with two,
   the colours wash out. During those frames the rig draws the whole
   drawing instead of its slices, the weapon at the skeleton's wrist.

## ComfyUI

Runs on the GM's GPU tower (RTX 3080 Ti, 12 GB). It needs, under
`ComfyUI/models/`:

| Folder | File |
|---|---|
| `checkpoints/` | `sd_xl_base_1.0.safetensors` |
| `loras/` | `pixel-art-xl.safetensors` (nerijs/pixel-art-xl) |
| `controlnet/` | `controlnet-union-sdxl-promax.safetensors` (xinsir/controlnet-union-sdxl-1.0) |
| `clip_vision/`, `ipadapter/` | IP-Adapter Plus SDXL (`ip-adapter-plus_sdxl_vit-h`, CLIP-ViT-H-14), for key poses |

ComfyUI has no authentication: its Windows firewall rule must only
admit the machines that call it. PCT 105 is not admitted yet, so
`generate` cannot run from there until the rule lists `192.168.1.192`.

## Commands

```bash
COMFYUI_URL=http://192.168.1.105:8188 bun run generate --gabarit moyen --name nain --seeds 1,2,3 "a stout dwarf…"
COMFYUI_URL=http://192.168.1.105:8188 bun run generate --item --name hache --seeds 1,2 "dwarven battle axe"
bun run src/cli.ts turnaround raw/nain_2.png --gabarit moyen --out views   # check the three views
bun run src/cli.ts animate cast.json --out anim                           # sheets for every direction
bun run src/cli.ts sheet preview.png anim/*.png                            # contact sheet
```

`cast.json` lists each character: `name`, `gabarit`, `sheet` (the
chosen turnaround), and optional `weapon` / `shield` items with their
`image`, `height` in art pixels and `grip` (share of the height from
the top where the hand closes). `keys` maps each key pose (`windup`,
`strike`, `cast`) to a two-panel drawing, the key pose on the right.

A sheet holds one row per animation (repos, marche, brandir, attaque,
soin, touché, mort) and one 80 px frame per column; `animations.json`
gives the frame counts and timings.
