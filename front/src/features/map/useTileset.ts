import { useEffect, useState } from 'react'
import type { MapData } from '@/lib/board'
import type { MediaList, Tileset } from '@/lib/media'

/**
 * The tileset `map` names in its world's theme, and the atlases the GM
 * approved for its materials, loaded as images (`imageUrl` builds the
 * GM's or the player's URL of an asset).
 */
export function useTileset(
  map: MapData | null,
  media: MediaList | null,
  imageUrl: (asset: string) => string,
): { tileset: Tileset | null; atlases: Record<string, HTMLImageElement> } {
  const tileset = (map && media?.theme?.tilesets.find((t) => t.id === map.theme)) ?? null
  const wanted = (media?.assets ?? [])
    .filter((a) => a.kind === 'tileset' && (a.status ?? 'approved') === 'approved' && tileset?.materials[a.subject])
    .map((a) => [a.subject, a.id] as const)
  const signature = wanted.map(([m, id]) => `${m}:${id}`).join('|')
  const [atlases, setAtlases] = useState<Record<string, HTMLImageElement>>({})

  useEffect(() => {
    let live = true
    const next: Record<string, HTMLImageElement> = {}
    for (const [material, id] of wanted) {
      const img = new Image()
      img.onload = () => {
        if (!live) return
        next[material] = img
        setAtlases({ ...next })
      }
      img.src = imageUrl(id)
    }
    return () => {
      live = false
    }
    // `signature` is the content of `wanted`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature])

  return { tileset, atlases }
}
