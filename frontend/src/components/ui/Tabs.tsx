import { NavLink } from 'react-router'

// Below lg the tabs stay on one line and scroll sideways; the bottom rule is
// an inset shadow so the scrolling row is not clipped by a 1px overflow.
const BAR =
  'flex gap-1 overflow-x-auto shadow-[inset_0_-1px_0_var(--color-border)] [scrollbar-width:none] lg:flex-wrap lg:overflow-visible [&::-webkit-scrollbar]:hidden'
const TAB = 'flex min-h-11 shrink-0 items-center border-b-2 px-3 text-sm font-medium whitespace-nowrap lg:min-h-0 lg:py-2'

/** Section tabs as links, so each tab has its own URL. */
export function Tabs({ items, label }: { items: { to: string; label: string; end?: boolean }[]; label: string }) {
  return (
    <nav aria-label={label} className={BAR}>
      {items.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.end ?? true}
          className={({ isActive }) => `${TAB} ${isActive ? 'border-brand-600 text-brand-700' : 'border-transparent text-ink-muted hover:text-ink'}`}
        >
          {item.label}
        </NavLink>
      ))}
    </nav>
  )
}

/** Tabs kept in component state (no URL). */
export function StateTabs<T extends string>({
  items,
  value,
  onChange,
  label,
}: {
  items: { value: T; label: string }[]
  value: T
  onChange: (value: T) => void
  label: string
}) {
  return (
    <div role="tablist" aria-label={label} className={BAR}>
      {items.map((item) => (
        <button
          key={item.value}
          type="button"
          role="tab"
          aria-selected={item.value === value}
          onClick={() => onChange(item.value)}
          className={`${TAB} ${item.value === value ? 'border-brand-600 text-brand-700' : 'border-transparent text-ink-muted hover:text-ink'}`}
        >
          {item.label}
        </button>
      ))}
    </div>
  )
}
