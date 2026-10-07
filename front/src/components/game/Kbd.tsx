/** The key a card or a button answers to, shown on a computer (player/play-on-desktop). */
export function Kbd({ children }: { children: string }) {
  return (
    <kbd className="mt-0.5 rounded-[4px] border border-line px-1.5 font-sans text-[10px] leading-4 text-mute-soft">{children}</kbd>
  )
}
