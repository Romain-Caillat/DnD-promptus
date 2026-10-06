/**
 * Line icons of the design canvas: `UI_ICONS` for buttons and the arcade
 * cluster (board « Composant — bouton-carte »), `CARD_ICONS` for the art
 * of a playing card (board « Composant — carte à jouer »). Each is a list
 * of SVG paths on a 24 × 24 grid; a circle is written as a path too.
 */

const circle = (cx: number, cy: number, r: number) =>
  `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`

export const UI_ICONS = {
  hourglass: [
    'M5 22h14M5 2h14M17 22v-4.172a2 2 0 0 0-.586-1.414L12 12l-4.414 4.414A2 2 0 0 0 7 17.828V22M7 2v4.172a2 2 0 0 0 .586 1.414L12 12l4.414-4.414A2 2 0 0 0 17 6.172V2',
  ],
  sword: ['M14.5 17.5 3 6V3h3l11.5 11.5M13 19l6-6M16 16l4 4M19 21l2-2'],
  shield: [
    'M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z',
  ],
  flask: ['M10 2v7.5L4.5 19A2 2 0 0 0 6.2 22h11.6a2 2 0 0 0 1.7-3L14 9.5V2M8.5 2h7M7 16h10'],
  send: ['M22 2 11 13M22 2l-7 20-4-9-9-4z'],
  eye: ['M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z'],
  link: [
    'M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71',
  ],
  stop: ['M6 6h12v12H6z'],
  plus: ['M12 5v14M5 12h14'],
  arrow: ['M5 12h14M13 6l6 6-6 6'],
  image: ['M3 5h18v14H3z', 'M3 16l5-5 4 4 3-3 6 6', circle(15.5, 9, 1.5)],
} as const satisfies Record<string, readonly string[]>

export type UiIcon = keyof typeof UI_ICONS

export const CARD_ICONS = {
  sword: ['M14.5 3H21v6.5L10.5 20 4 13.5z', 'M6.5 16l-3.5 3.5', 'M8 11l5 5'],
  shield: ['M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z', 'M12 7v10'],
  eye: ['M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z', circle(12, 12, 3)],
  speech: ['M4 5h16v11H10l-5 4v-4H4z', 'M8 10h8'],
  steps: ['M4 16c2-1 3-4 3-7a2 2 0 0 1 4 0c0 4-3 6-7 7z', 'M13 20c2-1 3-4 3-7a2 2 0 0 1 4 0c0 4-3 6-7 7z'],
  spell: [
    'M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z',
    'M19 17l.7 2 2 .7-2 .7-.7 2-.7-2-2-.7 2-.7z',
  ],
  skull: [
    'M5 11a7 7 0 0 1 14 0c0 3-1.5 4.5-3 5.5V20H8v-3.5C6.5 15.5 5 14 5 11z',
    circle(9.5, 11, 1.6),
    circle(14.5, 11, 1.6),
    'M11 20v-2',
    'M13 20v-2',
  ],
  scroll: ['M6 3h11v14a3 3 0 0 1-3 3H6z', 'M14 20a3 3 0 0 0 3-3', 'M9 8h5', 'M9 12h5'],
  door: ['M6 21V5a6 6 0 0 1 12 0v16z', 'M4 21h16', circle(14.5, 13, 1)],
} as const satisfies Record<string, readonly string[]>

export type CardIcon = keyof typeof CARD_ICONS

export function LineIcon({
  paths,
  size,
  strokeWidth,
}: {
  paths: readonly string[]
  size: number
  strokeWidth: number
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      {paths.map((d) => (
        <path key={d} d={d} />
      ))}
    </svg>
  )
}
