// Building blocks shared by every report page: the default date range, the
// printable report header (REP-R01), CSV/Print actions and query states.

import { type ReactNode, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { formatDateTime } from '../../lib/dates'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'

/** Business name, period and "generated at … by …" — printed with the report. */
export function ReportHeader({ period, updatedAt }: { period: ReactNode; updatedAt?: number }) {
  const { t, i18n } = useTranslation()
  const { me } = useSession()
  const [mounted] = useState(() => Date.now())
  const time = new Date(updatedAt || mounted).toISOString()
  return (
    <div className="flex flex-col gap-0.5 rounded-xl border border-border bg-surface px-4 py-3 text-sm print:border-0 print:px-0">
      <p className="text-base font-semibold">{me?.tenant?.name}</p>
      <p>{period}</p>
      <p className="text-xs text-ink-muted">{t('common.generatedAt', { time: formatDateTime(time, i18n.language), name: me?.user.full_name ?? '' })}</p>
    </div>
  )
}

export interface ExportAction {
  label?: string
  run: () => void
}

/**
 * A report page: title with CSV/Print buttons, filters (hidden in print),
 * the printable header and the report body. Without permission only a
 * notice is shown.
 */
export function ReportFrame({
  title,
  allowed,
  period,
  filters,
  exports = [],
  links,
  updatedAt,
  children,
}: {
  title: string
  allowed: boolean
  period: ReactNode
  filters?: ReactNode
  exports?: ExportAction[]
  links?: ReactNode
  updatedAt?: number
  children: ReactNode
}) {
  const { t } = useTranslation()
  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageHeader
        title={title}
        actions={
          allowed && (
            <div className="flex flex-wrap items-center gap-2 print:hidden">
              {links}
              {exports.map((e, i) => (
                <Button key={i} variant="secondary" onClick={e.run}>
                  {e.label ? `${t('common.exportCsv')} (${e.label})` : t('common.exportCsv')}
                </Button>
              ))}
              <Button variant="secondary" onClick={() => window.print()}>
                {t('common.print')}
              </Button>
            </div>
          )
        }
      />
      {!allowed ? (
        <Alert tone="warning">{t('reports.noAccess')}</Alert>
      ) : (
        <>
          {filters && <div className="flex flex-wrap items-end gap-3 print:hidden">{filters}</div>}
          <ReportHeader period={period} updatedAt={updatedAt} />
          {children}
        </>
      )}
    </div>
  )
}

/** Spinner while loading, a translated alert on error, else the content. */
export function Loaded<T>({
  query,
  children,
}: {
  query: { isPending: boolean; isError: boolean; error: unknown; data: T | undefined }
  children: (data: T) => ReactNode
}) {
  const { t } = useTranslation()
  if (query.isError) return <Alert tone="danger">{problemMessage(query.error, t)}</Alert>
  if (query.isPending || query.data === undefined) return <Spinner />
  return <>{children(query.data)}</>
}

/** A titled report section. */
export function Section({ title, children, actions }: { title: string; children: ReactNode; actions?: ReactNode }) {
  return (
    <section className="flex flex-col gap-3 print:break-inside-avoid">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-lg font-semibold">{title}</h2>
        {actions}
      </div>
      {children}
    </section>
  )
}

/** A "Balanced" / "Out of balance" badge with a text label (not color only). */
export function BalanceBadge({ balanced }: { balanced: boolean }) {
  const { t } = useTranslation()
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-lg px-3 py-1 text-sm font-semibold ${balanced ? 'bg-brand-50 text-brand-700' : 'bg-danger/10 text-danger'}`}
    >
      <span aria-hidden>{balanced ? '✓' : '!'}</span>
      {balanced ? t('reports.balanced') : t('reports.outOfBalance')}
    </span>
  )
}
