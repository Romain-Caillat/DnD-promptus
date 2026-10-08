import { useTranslation } from 'react-i18next'
import { THEME_CHOICES, setThemeChoice, useThemeChoice } from '@/lib/theme'
import { cn } from '@/lib/utils'

/**
 * « Thème : Sombre / Clair / Comme l'appareil » (ui/offer-light-theme):
 * three small chips, kept on this device. On the GM's home header and
 * next to the rules entry on the player's screen.
 */
export function ThemeSetting({ className }: { className?: string }) {
  const { t } = useTranslation()
  const choice = useThemeChoice()
  return (
    <div
      role="radiogroup"
      aria-label={t('theme.label')}
      className={cn('flex flex-wrap items-center gap-1.5 text-caption', className)}
    >
      <span className="mr-1 text-mute-soft">{t('theme.label')}</span>
      {THEME_CHOICES.map((c) => (
        <button
          key={c}
          type="button"
          role="radio"
          aria-checked={choice === c}
          onClick={() => setThemeChoice(c)}
          className={cn(
            'rounded-md px-2.5 py-1 font-semibold',
            choice === c ? 'bg-ivory text-ink shadow-ivory-flat' : 'border border-line text-chalk-soft',
          )}
        >
          {t(`theme.${c}`)}
        </button>
      ))}
    </div>
  )
}
