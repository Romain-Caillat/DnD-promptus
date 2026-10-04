import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import { defaultNS, resources } from './resources'

void i18n.use(initReactI18next).init({
  resources,
  lng: 'fr',
  fallbackLng: 'fr',
  defaultNS,
  interpolation: {
    // React already escapes rendered strings.
    escapeValue: false,
  },
})
