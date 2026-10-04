import { afterEach, describe, expect, it, vi } from 'vitest'
import { bytes, stubPasskeys } from '@/test-utils'
import { base64urlToBuffer, bufferToBase64url, createPasskey, getPasskey } from './webauthn'

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('webauthn bridge', () => {
  it('base64url round-trips bytes that need - and _ and no padding', () => {
    const raw = bytes(0xfb, 0xff, 0xbf, 0x00, 0x01)
    const text = bufferToBase64url(raw)
    expect(text).toBe('-_-_AAE')
    expect(new Uint8Array(base64urlToBuffer(text))).toEqual(new Uint8Array(raw))
  })

  it('registration: decodes the server challenge, encodes the answer the server reads', async () => {
    const create = vi.fn().mockResolvedValue({
      id: 'AQI',
      rawId: bytes(1, 2),
      type: 'public-key',
      response: { attestationObject: bytes(3), clientDataJSON: bytes(4) },
    })
    stubPasskeys({ create })
    const out = await createPasskey({
      publicKey: {
        challenge: 'AAE',
        user: { id: 'Ag', name: 'Romain', displayName: 'Romain' },
        rp: { id: 'localhost', name: 'Promptus' },
        excludeCredentials: [{ type: 'public-key', id: 'Aw' }],
      },
    })
    const pk = create.mock.calls[0][0].publicKey
    expect(new Uint8Array(pk.challenge)).toEqual(new Uint8Array([0, 1]))
    expect(new Uint8Array(pk.user.id)).toEqual(new Uint8Array([2]))
    expect(new Uint8Array(pk.excludeCredentials[0].id)).toEqual(new Uint8Array([3]))
    expect(out).toEqual({
      id: 'AQI',
      rawId: 'AQI',
      type: 'public-key',
      response: { attestationObject: 'Aw', clientDataJSON: 'BA' },
    })
  })

  it('sign-in: an empty allow list stays discoverable, the user handle comes back', async () => {
    const get = vi.fn().mockResolvedValue({
      id: 'AQ',
      rawId: bytes(1),
      type: 'public-key',
      response: {
        authenticatorData: bytes(5),
        clientDataJSON: bytes(6),
        signature: bytes(7),
        userHandle: bytes(8),
      },
    })
    stubPasskeys({ get })
    const out = await getPasskey({ publicKey: { challenge: 'AAE', allowCredentials: [] } })
    expect(get.mock.calls[0][0].publicKey.allowCredentials).toBeUndefined()
    expect(out.response).toEqual({
      authenticatorData: 'BQ',
      clientDataJSON: 'Bg',
      signature: 'Bw',
      userHandle: 'CA',
    })
  })
})
