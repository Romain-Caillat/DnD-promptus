import 'i18next'
import type { defaultNS, resources } from './resources'

// Type-checks every `t('…')` key against fr.json: a typo or a removed
// key fails `tsc` instead of rendering the raw key to a player.
declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: typeof defaultNS
    resources: (typeof resources)['fr']
  }
}
