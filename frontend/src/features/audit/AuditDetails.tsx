import { useTranslation } from 'react-i18next'

import type { AuditRecord } from '../../api/audit'
import { type ChangeKind, diffObjects, showValue } from './diff'

const ROW_TONE: Record<ChangeKind, string> = {
  added: 'bg-brand-50',
  removed: 'bg-danger/10',
  changed: 'bg-warning/10',
  same: '',
}

/** The expanded row: before/after as a key-by-key diff, plus request metadata. */
export function AuditDetails({ record }: { record: AuditRecord }) {
  const { t } = useTranslation()
  const rows = diffObjects(record.before, record.after)
  const hasData = !(record.before == null && record.after == null)

  return (
    <div className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">{t('audit.details')}</h3>
      {!hasData ? (
        <p className="text-sm text-ink-muted">{t('audit.noData')}</p>
      ) : rows ? (
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm">
            <caption className="sr-only">{t('audit.changes')}</caption>
            <thead className="text-xs text-ink-muted uppercase">
              <tr>
                <th scope="col" className="px-2 py-1.5">{t('audit.field')}</th>
                <th scope="col" className="px-2 py-1.5">{t('audit.before')}</th>
                <th scope="col" className="px-2 py-1.5">{t('audit.after')}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.key} className={`border-t border-border align-top ${ROW_TONE[row.kind]}`} data-change={row.kind}>
                  <th scope="row" className="px-2 py-1.5 font-mono text-xs font-medium">
                    {row.key}
                    {row.kind !== 'same' && <span className="ml-2 font-sans text-xs text-ink-muted">({t(`audit.change.${row.kind}`)})</span>}
                  </th>
                  <td className={`px-2 py-1.5 ${row.kind === 'changed' || row.kind === 'removed' ? 'line-through decoration-danger/60' : ''}`}>
                    <Value value={row.before} />
                  </td>
                  <td className={`px-2 py-1.5 ${row.kind === 'changed' || row.kind === 'added' ? 'font-semibold' : ''}`}>
                    <Value value={row.after} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          <div>
            <h4 className="text-xs font-semibold text-ink-muted uppercase">{t('audit.before')}</h4>
            <pre className="overflow-x-auto rounded bg-surface p-2 text-xs">{record.before == null ? '—' : JSON.stringify(record.before, null, 2)}</pre>
          </div>
          <div>
            <h4 className="text-xs font-semibold text-ink-muted uppercase">{t('audit.after')}</h4>
            <pre className="overflow-x-auto rounded bg-surface p-2 text-xs">{record.after == null ? '—' : JSON.stringify(record.after, null, 2)}</pre>
          </div>
        </div>
      )}
      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs text-ink-muted">
        {record.entity_id && (
          <>
            <dt>{t('audit.entityId')}</dt>
            <dd className="font-mono break-all">{record.entity_id}</dd>
          </>
        )}
        {record.request_id && (
          <>
            <dt>{t('audit.requestId')}</dt>
            <dd className="font-mono break-all">{record.request_id}</dd>
          </>
        )}
        {record.user_agent && (
          <>
            <dt>{t('audit.userAgent')}</dt>
            <dd className="break-all">{record.user_agent}</dd>
          </>
        )}
      </dl>
    </div>
  )
}

function Value({ value }: { value: unknown }) {
  if (value === undefined) return <span className="text-ink-muted">—</span>
  const text = showValue(value)
  return typeof value === 'object' && value !== null ? (
    <pre className="text-xs whitespace-pre-wrap">{text}</pre>
  ) : (
    <span className="break-all">{text}</span>
  )
}
