import { ApiError, apiRequest } from './api'

/** The signed-in GM. */
export interface Gm {
  id: string
  displayName: string
}

/** The signed-in GM, or `null` without a valid session. */
export async function fetchMe(): Promise<Gm | null> {
  try {
    return await apiRequest<Gm>('GET', '/me')
  } catch (err) {
    if (err instanceof ApiError && err.status === 401) return null
    throw err
  }
}

/** Email a sign-in code to `email`. */
export function requestCode(email: string): Promise<void> {
  return apiRequest<void>('POST', '/auth/code', { email })
}

/**
 * Sign in with the code received. The first time an address signs in,
 * the server asks for a display name (`DISPLAY_NAME_REQUIRED`) and keeps
 * the code: send it again with `displayName` to create the account.
 */
export function verifyCode(email: string, code: string, displayName?: string): Promise<Gm> {
  return apiRequest<Gm>('POST', '/auth/verify', {
    email,
    code,
    ...(displayName === undefined ? {} : { displayName }),
  })
}

export async function signOut(): Promise<void> {
  try {
    await apiRequest<void>('POST', '/auth/sign-out')
  } catch (err) {
    // Already signed out (expired session): the goal is reached.
    if (!(err instanceof ApiError && err.status === 401)) throw err
  }
}

const ERROR_KEYS = {
  INVALID_EMAIL: 'auth.errors.invalidEmail',
  INVALID_CODE: 'auth.errors.invalidCode',
  INVALID_DISPLAY_NAME: 'auth.errors.invalidDisplayName',
  CODE_TOO_SOON: 'auth.errors.codeTooSoon',
  TOO_MANY_CODES: 'auth.errors.tooManyCodes',
  EMAIL_UNAVAILABLE: 'auth.errors.emailUnavailable',
  UNREACHABLE: 'auth.errors.unreachable',
} as const

export type AuthErrorKey = (typeof ERROR_KEYS)[keyof typeof ERROR_KEYS] | 'auth.errors.generic'

/** The translation key explaining why a sign-in step failed. */
export function authErrorKey(err: unknown): AuthErrorKey {
  if (err instanceof ApiError && err.code in ERROR_KEYS) {
    return ERROR_KEYS[err.code as keyof typeof ERROR_KEYS]
  }
  return 'auth.errors.generic'
}
