/**
 * Dates as the French interface writes them, in the reader's own time
 * zone: the server stores instants, each screen shows them local.
 */

/** « 3 oct. » */
export function shortDate(iso: string): string {
  return new Date(iso).toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })
}
