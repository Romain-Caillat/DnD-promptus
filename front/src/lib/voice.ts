import { apiRequest } from './api'
import type { CopilotKind, Draft } from './evening'

/**
 * copilot/listen-by-voice — the GM's dictation, recorded in the browser
 * and sent as a 16 kHz mono 16-bit WAV. Every audio model takes WAV,
 * whereas what `MediaRecorder` makes differs by browser (WebM on Chrome,
 * MP4 on Safari): the samples are taken raw from Web Audio and encoded
 * here, so the server sees one format whatever the device.
 */

/** The rate the WAV is written at: enough for speech, a third of 48 kHz. */
const VOICE_RATE = 16_000
/** The longest dictation, in seconds (the server refuses beyond). */
export const MAX_SECONDS = 60

/** `samples` resampled from `from` Hz to `to` Hz, averaging each window. */
export function downsample(samples: Float32Array, from: number, to: number): Float32Array {
  if (to >= from) return samples
  const ratio = from / to
  const out = new Float32Array(Math.floor(samples.length / ratio))
  for (let i = 0; i < out.length; i++) {
    const start = Math.floor(i * ratio)
    const end = Math.min(samples.length, Math.floor((i + 1) * ratio))
    let sum = 0
    for (let j = start; j < end; j++) sum += samples[j]
    out[i] = end > start ? sum / (end - start) : 0
  }
  return out
}

/** A mono 16-bit PCM WAV file of `samples` (each in −1…1) at `rate` Hz. */
export function encodeWav(samples: Float32Array, rate: number): Uint8Array {
  const bytes = new Uint8Array(44 + samples.length * 2)
  const view = new DataView(bytes.buffer)
  const text = (at: number, s: string) => [...s].forEach((c, i) => view.setUint8(at + i, c.charCodeAt(0)))
  text(0, 'RIFF')
  view.setUint32(4, 36 + samples.length * 2, true)
  text(8, 'WAVEfmt ')
  view.setUint32(16, 16, true)
  view.setUint16(20, 1, true) // PCM
  view.setUint16(22, 1, true) // mono
  view.setUint32(24, rate, true)
  view.setUint32(28, rate * 2, true)
  view.setUint16(32, 2, true)
  view.setUint16(34, 16, true)
  text(36, 'data')
  view.setUint32(40, samples.length * 2, true)
  samples.forEach((s, i) => {
    const c = Math.max(-1, Math.min(1, s))
    view.setInt16(44 + i * 2, c < 0 ? c * 0x8000 : c * 0x7fff, true)
  })
  return bytes
}

export function toBase64(bytes: Uint8Array): string {
  let binary = ''
  for (let i = 0; i < bytes.length; i += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
  }
  return btoa(binary)
}

/** A dictation in progress. */
export interface Recording {
  /** Stop and give the WAV, base64, with its length. */
  stop: () => Promise<{ audio: string; seconds: number }>
  /** Stop and throw the recording away. */
  cancel: () => void
}

/** How the screen records: the browser's microphone, or a stand-in in tests. */
export interface Recorder {
  /** False without a microphone API (an HTTP page that is not localhost). */
  supported: boolean
  start: () => Promise<Recording>
}

type AudioContextClass = typeof AudioContext

function audioContextClass(): AudioContextClass | undefined {
  const w = globalThis as unknown as { AudioContext?: AudioContextClass; webkitAudioContext?: AudioContextClass }
  return w.AudioContext ?? w.webkitAudioContext
}

/** Record the microphone through Web Audio until `stop`. */
async function startBrowserRecording(): Promise<Recording> {
  const stream = await navigator.mediaDevices.getUserMedia({
    audio: { channelCount: 1, echoCancellation: true, noiseSuppression: true },
  })
  const Ctx = audioContextClass()!
  const ctx = new Ctx()
  const source = ctx.createMediaStreamSource(stream)
  // ScriptProcessor is deprecated but runs everywhere, Safari included,
  // without a separate worklet file; a minute of speech is little work.
  const processor = ctx.createScriptProcessor(4096, 1, 1)
  const chunks: Float32Array[] = []
  processor.onaudioprocess = (e) => chunks.push(new Float32Array(e.inputBuffer.getChannelData(0)))
  source.connect(processor)
  // Chrome only runs a processor that reaches the output; it writes silence.
  processor.connect(ctx.destination)
  const release = () => {
    processor.onaudioprocess = null
    source.disconnect()
    processor.disconnect()
    stream.getTracks().forEach((track) => track.stop())
    void ctx.close()
  }
  return {
    stop: async () => {
      const rate = ctx.sampleRate
      release()
      const all = new Float32Array(chunks.reduce((n, c) => n + c.length, 0))
      let at = 0
      for (const c of chunks) {
        all.set(c, at)
        at += c.length
      }
      const samples = downsample(all, rate, VOICE_RATE)
      return { audio: toBase64(encodeWav(samples, VOICE_RATE)), seconds: samples.length / VOICE_RATE }
    },
    cancel: release,
  }
}

export const browserRecorder: Recorder = {
  supported:
    typeof navigator !== 'undefined' &&
    typeof navigator.mediaDevices?.getUserMedia === 'function' &&
    audioContextClass() !== undefined,
  start: startBrowserRecording,
}

/** What the co-GM heard, and its draft. */
export interface Heard {
  transcript: string
  seconds: number
  draft: Draft
}

/** Send a dictation: it is written down, then asked to the co-GM (two counted AI calls). */
export function dictate(campaignId: string, audio: string, kind: CopilotKind, npc?: string): Promise<Heard> {
  return apiRequest<Heard>('POST', `/campaigns/${encodeURIComponent(campaignId)}/session/copilot/voice`, {
    audio,
    kind,
    ...(npc ? { npc } : {}),
  })
}
