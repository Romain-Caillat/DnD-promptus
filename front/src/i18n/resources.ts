import fr from './locales/fr.json'

/**
 * French is the reference locale (CLAUDE.md, rule 5): every key exists
 * in `fr.json` first. Other locales, if any ever come, translate from it.
 */
export const defaultNS = 'translation'

export const resources = {
  fr: { translation: fr },
} as const
