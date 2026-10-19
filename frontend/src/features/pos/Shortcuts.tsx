import { useTranslation } from 'react-i18next'

const KEYS = [
  ['F2', 'search'],
  ['F4', 'customer'],
  ['F7', 'discount'],
  ['F8', 'pay'],
  ['F9', 'hold'],
  ['F10', 'held'],
  ['Ctrl+Del', 'removeLine'],
  ['Esc', 'close'],
] as const

/** Small keyboard-shortcut legend. */
export function Shortcuts() {
  const { t } = useTranslation()
  return (
    <dl aria-label={t('pos.shortcuts.title')} className="mt-auto hidden grid-cols-2 gap-x-3 gap-y-0.5 text-xs text-ink-muted md:grid">
      {KEYS.map(([key, name]) => (
        <div key={key} className="flex gap-1.5">
          <dt>
            <kbd className="rounded border border-border bg-canvas px-1 font-mono">{key}</kbd>
          </dt>
          <dd>{t(`pos.shortcuts.${name}`)}</dd>
        </div>
      ))}
    </dl>
  )
}
