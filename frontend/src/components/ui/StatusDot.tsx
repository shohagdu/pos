export type Status = 'ok' | 'down' | 'pending'

const COLORS: Record<Status, string> = {
  ok: 'bg-success',
  down: 'bg-danger',
  pending: 'bg-warning animate-pulse',
}

export function StatusDot({ status, label }: { status: Status; label: string }) {
  return (
    <span className="inline-flex items-center gap-2 text-sm">
      <span aria-hidden className={`size-2.5 rounded-full ${COLORS[status]}`} />
      {label}
    </span>
  )
}
