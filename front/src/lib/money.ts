/** Dollars, two decimals, French style (`0,42`). */
export function dollars(micros: number): string {
  return (micros / 1_000_000).toLocaleString('fr-FR', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}
