import { clsx, type ClassValue } from 'clsx'
import { extendTailwindMerge } from 'tailwind-merge'

// tailwind-merge must know the custom token names of styles/tokens.css:
// otherwise it reads `text-label` as a colour and `cn('text-label
// text-chalk')` silently drops the size.
const twMerge = extendTailwindMerge({
  extend: {
    theme: {
      text: [
        'label',
        'caption',
        'body',
        'stat',
        'card-title',
        'heading',
        'heading-lg',
        'display',
        'narration',
        'narration-lg',
        'pixel',
      ],
      shadow: ['slab', 'ivory', 'ivory-pressed', 'ivory-flat', 'dark', 'dark-pressed', 'card', 'turn'],
      radius: ['frame', 'button', 'panel', 'slab'],
      font: ['title', 'narration', 'pixel'],
      ease: ['deal', 'settle', 'roll'],
      animate: ['deal', 'press-demo', 'pop', 'tumble', 'bob', 'sheen', 'holo'],
      'drop-shadow': ['gem'],
    },
  },
})

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
