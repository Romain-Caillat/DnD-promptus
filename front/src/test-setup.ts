import '@testing-library/jest-dom/vitest'
// Real French strings in tests: a missing key renders as the raw key and
// fails the assertion instead of passing silently.
import '@/i18n'
