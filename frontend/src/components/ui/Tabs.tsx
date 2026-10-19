import { NavLink } from 'react-router'

/** Section tabs as links, so each tab has its own URL. */
export function Tabs({ items, label }: { items: { to: string; label: string; end?: boolean }[]; label: string }) {
  return (
    <nav aria-label={label} className="flex flex-wrap gap-1 border-b border-border">
      {items.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.end ?? true}
          className={({ isActive }) =>
            `-mb-px border-b-2 px-3 py-2 text-sm font-medium ${isActive ? 'border-brand-600 text-brand-700' : 'border-transparent text-ink-muted hover:text-ink'}`
          }
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
    <div role="tablist" aria-label={label} className="flex flex-wrap gap-1 border-b border-border">
      {items.map((item) => (
        <button
          key={item.value}
          type="button"
          role="tab"
          aria-selected={item.value === value}
          onClick={() => onChange(item.value)}
          className={`-mb-px border-b-2 px-3 py-2 text-sm font-medium ${item.value === value ? 'border-brand-600 text-brand-700' : 'border-transparent text-ink-muted hover:text-ink'}`}
        >
          {item.label}
        </button>
      ))}
    </div>
  )
}
