// The three typefaces of MEMORY.md §2, served by the app itself (no font
// CDN: the app must work on a home server and inside Tauri offline).
// Latin subset only: it covers French, including œ, « » and the minus
// sign used by damage numbers. Weights are the ones the design canvas uses.

// Cinzel: titles, cards, button labels.
import '@fontsource/cinzel/latin-600.css'
import '@fontsource/cinzel/latin-700.css'
import '@fontsource/cinzel/latin-800.css'

// Cormorant Garamond italic: text read aloud (narration).
import '@fontsource/cormorant-garamond/latin-500-italic.css'
import '@fontsource/cormorant-garamond/latin-600-italic.css'

// Chakra Petch: interface and numbers.
import '@fontsource/chakra-petch/latin-400.css'
import '@fontsource/chakra-petch/latin-500.css'
import '@fontsource/chakra-petch/latin-600.css'
import '@fontsource/chakra-petch/latin-700.css'
