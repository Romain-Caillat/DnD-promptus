import { ApiError, apiRequest } from './api'
import { createPasskey, getPasskey, isPasskeyCancel, type Json } from './webauthn'

/** The signed-in GM. */
export interface Gm {
  id: string
  displayName: string
}

/** A pending invitation for another GM. The code is never listed. */
export interface GmInvite {
  id: string
  createdAt: string
  expiresAt: string
}

interface Challenge {
  ceremonyId: string
  options: Json
}

/** Whether the first GM account is still to be created. */
export async function fetchNeedsSetup(): Promise<boolean> {
  const status = await apiRequest<{ needsSetup: boolean }>('GET', '/auth/status')
  return status.needsSetup
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

/** Sign in with any passkey of this site; the server tells whose it is. */
export async function signInWithPasskey(): Promise<Gm> {
  const challenge = await apiRequest<Challenge>('POST', '/auth/sign-in/options')
  const credential = await getPasskey(challenge.options)
  return apiRequest<Gm>('POST', '/auth/sign-in', {
    ceremonyId: challenge.ceremonyId,
    credential,
  })
}

/**
 * Create a GM account with a new passkey. `code` is the setup code of a
 * fresh instance or an invitation from another GM.
 */
export async function registerWithPasskey(code: string, displayName: string): Promise<Gm> {
  const challenge = await apiRequest<Challenge>('POST', '/auth/register/options', {
    code,
    displayName,
  })
  const credential = await createPasskey(challenge.options)
  return apiRequest<Gm>('POST', '/auth/register', {
    ceremonyId: challenge.ceremonyId,
    credential,
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

export function listInvites(): Promise<GmInvite[]> {
  return apiRequest<GmInvite[]>('GET', '/gm-invites')
}

export function createInvite(): Promise<GmInvite & { code: string }> {
  return apiRequest<GmInvite & { code: string }>('POST', '/gm-invites')
}

export function revokeInvite(id: string): Promise<void> {
  return apiRequest<void>('DELETE', `/gm-invites/${encodeURIComponent(id)}`)
}

/** The link that opens account creation with an invitation code. */
export function inviteLink(code: string): string {
  return `${window.location.origin}/inscription?code=${encodeURIComponent(code)}`
}

const ERROR_KEYS = {
  INVALID_REGISTRATION_CODE: 'auth.errors.invalidCode',
  INVALID_DISPLAY_NAME: 'auth.errors.invalidDisplayName',
  INVALID_PASSKEY: 'auth.errors.invalidPasskey',
  PASSKEY_ALREADY_REGISTERED: 'auth.errors.passkeyTaken',
  UNREACHABLE: 'auth.errors.unreachable',
} as const

export type AuthErrorKey = (typeof ERROR_KEYS)[keyof typeof ERROR_KEYS] | 'auth.errors.cancelled' | 'auth.errors.generic'

/** The translation key explaining why a sign-in step failed. */
export function authErrorKey(err: unknown): AuthErrorKey {
  if (isPasskeyCancel(err)) return 'auth.errors.cancelled'
  if (err instanceof ApiError && err.code in ERROR_KEYS) {
    return ERROR_KEYS[err.code as keyof typeof ERROR_KEYS]
  }
  return 'auth.errors.generic'
}
