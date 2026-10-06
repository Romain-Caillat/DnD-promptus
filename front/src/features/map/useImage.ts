import { useEffect, useState } from 'react'

/** `src` loaded as an image, or null until it is (and when `src` is null). */
export function useImage(src: string | null): HTMLImageElement | null {
  const [loaded, setLoaded] = useState<{ src: string; img: HTMLImageElement } | null>(null)

  useEffect(() => {
    if (!src) return
    let live = true
    const img = new Image()
    img.onload = () => {
      if (live) setLoaded({ src, img })
    }
    img.src = src
    return () => {
      live = false
    }
  }, [src])

  return loaded && loaded.src === src ? loaded.img : null
}
