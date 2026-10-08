// The four typefaces of MEMORY.md (visual direction), served by the app
// itself (no font CDN: the app must work on a home server and inside Tauri
// offline). Latin subset only: it covers French, including œ, « » and the
// minus sign used by damage numbers. Weights are the ones the design uses.

// Silkscreen (SIL Open Font License 1.1): the pixel menu — titles, buttons,
// labels, tabs (ui/adopt-pixel-menu). Its glyphs sit on a grid of 1/8 em:
// crisp at 16 and 24px, and at 12px on a 2× screen. It has no ▶ and no −,
// which is why the menu cursor is drawn in cells and numbers stay in
// Chakra Petch.
import '@fontsource/silkscreen/latin-400.css'
import '@fontsource/silkscreen/latin-700.css'

// Cinzel: the playing cards (skill cards, their emblem, the threat clock).
import '@fontsource/cinzel/latin-600.css'
import '@fontsource/cinzel/latin-700.css'
import '@fontsource/cinzel/latin-800.css'

// Cormorant Garamond italic: text read aloud (narration).
import '@fontsource/cormorant-garamond/latin-500-italic.css'
import '@fontsource/cormorant-garamond/latin-600-italic.css'

// Chakra Petch: running text and numbers.
import '@fontsource/chakra-petch/latin-400.css'
import '@fontsource/chakra-petch/latin-500.css'
import '@fontsource/chakra-petch/latin-600.css'
import '@fontsource/chakra-petch/latin-700.css'
