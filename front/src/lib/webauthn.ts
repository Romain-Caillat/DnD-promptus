/**
 * platform/sign-in-gm — the bridge between the server's WebAuthn JSON
 * (webauthn-rs: binary fields as unpadded base64url) and the browser's
 * `navigator.credentials`, which speaks ArrayBuffers. Same bridge as
 * Devotion's, without the PRF extension.
 *
 * The server's challenge is `{ publicKey: {...} }`; the answer sent back
 * is `{ id, rawId, type, response: {...} }` with every binary field
 * base64url-encoded. Extension results and transports are left out: the
 * server does not need them.
 */

/** Whether this browser can use passkeys here (secure context + API). */
export function passkeysSupported(): boolean {
  return (
    typeof window !== 'undefined' &&
    window.isSecureContext === true &&
    typeof window.PublicKeyCredential === 'function' &&
    typeof navigator !== 'undefined' &&
    !!navigator.credentials
  )
}

export function base64urlToBuffer(value: string): ArrayBuffer {
  const b64 = value.replace(/-/g, '+').replace(/_/g, '/')
  const padded = b64 + '='.repeat((4 - (b64.length % 4)) % 4)
  const bin = atob(padded)
  const bytes = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i)
  return bytes.buffer
}

export function bufferToBase64url(value: ArrayBuffer | ArrayBufferView): string {
  const bytes =
    value instanceof ArrayBuffer
      ? new Uint8Array(value)
      : new Uint8Array(value.buffer, value.byteOffset, value.byteLength)
  let bin = ''
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i])
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

/* eslint-disable @typescript-eslint/no-explicit-any */
export type Json = Record<string, any>

function descriptors(list: unknown): PublicKeyCredentialDescriptor[] | undefined {
  if (!Array.isArray(list)) return undefined
  return list.map((c: Json) => ({
    type: 'public-key' as const,
    id: base64urlToBuffer(c.id),
    ...(Array.isArray(c.transports) ? { transports: c.transports } : {}),
  }))
}

/** Create a passkey from a server registration challenge. */
export async function createPasskey(options: Json): Promise<Json> {
  const pk = options.publicKey as Json
  const publicKey: PublicKeyCredentialCreationOptions = {
    ...(pk as PublicKeyCredentialCreationOptions),
    challenge: base64urlToBuffer(pk.challenge),
    user: { ...pk.user, id: base64urlToBuffer(pk.user.id) },
    excludeCredentials: descriptors(pk.excludeCredentials),
  }
  const cred = (await navigator.credentials.create({ publicKey })) as PublicKeyCredential | null
  if (!cred) throw new PasskeyCancelled()
  const response = cred.response as AuthenticatorAttestationResponse
  return {
    id: cred.id,
    rawId: bufferToBase64url(cred.rawId),
    type: cred.type,
    response: {
      attestationObject: bufferToBase64url(response.attestationObject),
      clientDataJSON: bufferToBase64url(response.clientDataJSON),
    },
  }
}

/** Answer a server sign-in challenge with a passkey. */
export async function getPasskey(options: Json): Promise<Json> {
  const pk = options.publicKey as Json
  const allow = descriptors(pk.allowCredentials)
  const publicKey: PublicKeyCredentialRequestOptions = {
    ...(pk as PublicKeyCredentialRequestOptions),
    challenge: base64urlToBuffer(pk.challenge),
    // An empty list means "any passkey of this site" (discoverable).
    allowCredentials: allow && allow.length > 0 ? allow : undefined,
  }
  const cred = (await navigator.credentials.get({ publicKey })) as PublicKeyCredential | null
  if (!cred) throw new PasskeyCancelled()
  const response = cred.response as AuthenticatorAssertionResponse
  return {
    id: cred.id,
    rawId: bufferToBase64url(cred.rawId),
    type: cred.type,
    response: {
      authenticatorData: bufferToBase64url(response.authenticatorData),
      clientDataJSON: bufferToBase64url(response.clientDataJSON),
      signature: bufferToBase64url(response.signature),
      userHandle: response.userHandle ? bufferToBase64url(response.userHandle) : null,
    },
  }
}
/* eslint-enable @typescript-eslint/no-explicit-any */

/** The person closed the prompt or the authenticator gave nothing. */
class PasskeyCancelled extends Error {
  constructor() {
    super('The passkey prompt was closed.')
    this.name = 'PasskeyCancelled'
  }
}

/**
 * True when `err` is the browser's "cancelled / timed out / not allowed"
 * (a `NotAllowedError` DOMException) or our own cancel.
 */
export function isPasskeyCancel(err: unknown): boolean {
  if (err instanceof PasskeyCancelled) return true
  return (
    typeof err === 'object' &&
    err !== null &&
    (err as { name?: unknown }).name === 'NotAllowedError'
  )
}
