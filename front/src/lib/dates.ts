/**
 * Dates as the French interface writes them, in the reader's own time
 * zone: the server stores instants, each screen shows them local.
 */

/** « 3 oct. » */
export function shortDate(iso: string): string {
  return new Date(iso).toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })
}

/** « 20 h 30 », « 21 h » */
export function clock(iso: string): string {
  const d = new Date(iso)
  const m = d.getMinutes()
  return m === 0 ? `${d.getHours()} h` : `${d.getHours()} h ${String(m).padStart(2, '0')}`
}

/** « jeudi 10 octobre · 20 h 30 » */
export function dayAndTime(iso: string): string {
  const day = new Date(iso).toLocaleDateString('fr-FR', { weekday: 'long', day: 'numeric', month: 'long' })
  return `${day} · ${clock(iso)}`
}

/** An instant as a `datetime-local` input shows it, in the reader's zone. */
export function toLocalInput(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
}
