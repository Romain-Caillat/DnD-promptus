import { describe, expect, it } from 'vitest'
import { downsample, encodeWav, toBase64 } from './voice'

describe('the dictation recording', () => {
  it('is a mono 16-bit PCM WAV the server can time', () => {
    const wav = encodeWav(new Float32Array([0, 1, -1, 2, 0.5]), 16_000)
    const view = new DataView(wav.buffer)
    const text = (at: number, n: number) => String.fromCharCode(...wav.subarray(at, at + n))
    expect(text(0, 4)).toBe('RIFF')
    expect(text(8, 8)).toBe('WAVEfmt ')
    expect(view.getUint16(20, true)).toBe(1)
    expect(view.getUint16(22, true)).toBe(1)
    expect(view.getUint32(24, true)).toBe(16_000)
    expect(view.getUint32(28, true)).toBe(32_000)
    expect(view.getUint16(34, true)).toBe(16)
    expect(text(36, 4)).toBe('data')
    expect(view.getUint32(40, true)).toBe(10)
    expect(wav.length).toBe(54)
    // Full scale both ways; beyond it is clipped, not wrapped.
    expect([0, 1, 2, 3, 4].map((i) => view.getInt16(44 + i * 2, true))).toEqual([0, 32767, -32768, 32767, 16383])
  })

  it('is brought down to 16 kHz by averaging, keeping its length in time', () => {
    const second = new Float32Array(48_000).map((_, i) => (i % 3 === 0 ? 0.3 : 0))
    const out = downsample(second, 48_000, 16_000)
    expect(out.length).toBe(16_000)
    expect(out[0]).toBeCloseTo(0.1)
    // Already at the rate: unchanged.
    expect(downsample(out, 16_000, 16_000)).toBe(out)
  })

  it('travels as base64, even past the size one call can spread', () => {
    expect(toBase64(new Uint8Array([82, 73, 70, 70]))).toBe('UklGRg==')
    const big = new Uint8Array(100_000).fill(65)
    expect(atob(toBase64(big)).length).toBe(100_000)
  })
})
