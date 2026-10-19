import { type FormEvent, Fragment, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type AuditFilter, type AuditRecord, useAuditLogs } from '../../api/audit'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { downloadCsv } from '../../lib/csv'
import { formatDateTime } from '../../lib/dates'
import { problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { AuditDetails } from './AuditDetails'
import { ENTITY_TYPES, dateRange } from './filters'

const ACTOR_TONE = { user: 'neutral', platform_staff: 'warn', system: 'info', api_key: 'info' } as const

const shortId = (id: string | null) => (id ? id.slice(0, 8) : '')

/** AUD-R03: who did what, when, newest first, with before/after details. */
export function AuditLogPage() {
  const { t } = useTranslation()
  const { me, can } = useSession()
  if (!can('audit.view')) {
    return (
      <div className="flex max-w-6xl flex-col gap-6">
        <PageHeader title={t('audit.title')} />
        <Alert tone="warning">{t('audit.noPermission')}</Alert>
      </div>
    )
  }
  return <AuditLog tenantId={me?.tenant?.tenant_id ?? ''} />
}

function AuditLog({ tenantId }: { tenantId: string }) {
  const { t, i18n } = useTranslation()

  const [action, setAction] = useState('')
  const [entity, setEntity] = useState('')
  const [range, setRange] = useState({ from: '', to: '' })
  const [actor, setActor] = useState<{ id: string; name: string } | null>(null)
  const [applied, setApplied] = useState({ action: '', entity: '' })
  const [open, setOpen] = useState<string | null>(null)

  const filter: AuditFilter = {
    action: applied.action || undefined,
    entity_type: applied.entity || undefined,
    actor_id: actor?.id,
    ...dateRange(range.from, range.to),
  }
  const logs = useAuditLogs(tenantId, filter)
  const rows = logs.data?.pages.flatMap((p) => p.data) ?? []

  function onApply(e: FormEvent) {
    e.preventDefault()
    setApplied({ action: action.trim(), entity: entity.trim() })
  }

  function clear() {
    setAction('')
    setEntity('')
    setApplied({ action: '', entity: '' })
    setRange({ from: '', to: '' })
    setActor(null)
  }

  const actorName = (r: AuditRecord) => r.actor_name ?? t(`audit.actorTypes.${r.actor_type}`)

  function exportCsv() {
    downloadCsv(`audit-log-${new Date().toISOString().slice(0, 10)}`, rows, [
      { header: t('audit.time'), value: (r) => r.created_at },
      { header: t('audit.actor'), value: (r) => r.actor_name ?? '' },
      { header: t('audit.actorType'), value: (r) => t(`audit.actorTypes.${r.actor_type}`) },
      { header: t('audit.action'), value: (r) => r.action },
      { header: t('audit.entityType'), value: (r) => r.entity_type },
      { header: t('audit.entityId'), value: (r) => r.entity_id },
      { header: t('common.reason'), value: (r) => r.reason },
      { header: t('audit.ip'), value: (r) => r.ip_address },
      { header: t('audit.before'), value: (r) => (r.before === null || r.before === undefined ? '' : JSON.stringify(r.before)) },
      { header: t('audit.after'), value: (r) => (r.after === null || r.after === undefined ? '' : JSON.stringify(r.after)) },
    ])
  }

  return (
    <div className="flex max-w-6xl flex-col gap-6">
      <PageHeader
        title={t('audit.title')}
        subtitle={t('audit.subtitle')}
        actions={
          <Button variant="secondary" disabled={rows.length === 0} onClick={exportCsv}>
            {t('common.exportCsv')}
          </Button>
        }
      />

      <div className="flex flex-col gap-3">
        <form onSubmit={onApply} className="flex flex-wrap items-end gap-3" role="search" aria-label={t('audit.filters')}>
          <TextField
            className="min-w-52 flex-1"
            label={t('audit.action')}
            value={action}
            onChange={(e) => setAction(e.target.value)}
            placeholder="sale.*"
            hint={t('audit.actionHint')}
          />
          <TextField
            className="min-w-52 flex-1"
            label={t('audit.entityType')}
            value={entity}
            onChange={(e) => setEntity(e.target.value)}
            list="audit-entity-types"
            hint={t('audit.entityHint')}
          />
          <datalist id="audit-entity-types">
            {ENTITY_TYPES.map((e) => (
              <option key={e} value={e} />
            ))}
          </datalist>
          <Button type="submit">{t('common.apply')}</Button>
        </form>
        <div className="flex flex-wrap items-end justify-between gap-3">
          <DateRangeFilter from={range.from} to={range.to} onChange={setRange} />
          <Button variant="ghost" onClick={clear}>
            {t('common.clear')}
          </Button>
        </div>
        {actor && (
          <div className="flex flex-wrap items-center gap-2 text-sm">
            <span>{t('audit.byActor', { name: actor.name })}</span>
            <Button variant="ghost" onClick={() => setActor(null)} aria-label={t('audit.clearActor')}>
              ✕
            </Button>
          </div>
        )}
      </div>

      {logs.isPending ? (
        <Spinner />
      ) : logs.isError ? (
        <Alert tone="danger">{problemMessage(logs.error, t)}</Alert>
      ) : rows.length === 0 ? (
        <p className="text-sm text-ink-muted">{t('audit.none')}</p>
      ) : (
        <div className="overflow-x-auto rounded-xl border border-border bg-surface">
          <table className="w-full text-left text-sm">
            <caption className="sr-only">{t('audit.title')}</caption>
            <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
              <tr>
                <th scope="col" className="px-3 py-2.5 whitespace-nowrap">{t('audit.time')}</th>
                <th scope="col" className="px-3 py-2.5">{t('audit.actor')}</th>
                <th scope="col" className="px-3 py-2.5">{t('audit.action')}</th>
                <th scope="col" className="px-3 py-2.5">{t('audit.entity')}</th>
                <th scope="col" className="px-3 py-2.5">{t('common.reason')}</th>
                <th scope="col" className="px-3 py-2.5">{t('audit.ip')}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => {
                const expanded = open === r.id
                const toggle = () => setOpen(expanded ? null : r.id)
                return (
                  <Fragment key={r.id}>
                    <tr onClick={toggle} className={`cursor-pointer border-b border-border hover:bg-canvas ${expanded ? 'bg-canvas' : ''}`}>
                      <td className="px-3 py-2.5 whitespace-nowrap">
                        <button
                          type="button"
                          aria-expanded={expanded}
                          aria-controls={`audit-${r.id}`}
                          onClick={(e) => {
                            e.stopPropagation()
                            toggle()
                          }}
                          className="text-left hover:underline"
                        >
                          <span aria-hidden className="mr-1 inline-block w-3 text-ink-muted">
                            {expanded ? '▾' : '▸'}
                          </span>
                          {formatDateTime(r.created_at, i18n.language)}
                          <span className="sr-only"> — {t('audit.details')}</span>
                        </button>
                      </td>
                      <td className="px-3 py-2.5">
                        <div className="flex flex-wrap items-center gap-2">
                          {r.actor_id ? (
                            <button
                              type="button"
                              className="font-medium text-brand-700 hover:underline"
                              title={t('audit.filterByActor')}
                              onClick={(e) => {
                                e.stopPropagation()
                                setActor({ id: r.actor_id!, name: actorName(r) })
                              }}
                            >
                              {actorName(r)}
                            </button>
                          ) : (
                            <span>{actorName(r)}</span>
                          )}
                          <Badge tone={ACTOR_TONE[r.actor_type] ?? 'neutral'}>{t(`audit.actorTypes.${r.actor_type}`)}</Badge>
                        </div>
                      </td>
                      <td className="px-3 py-2.5 font-mono text-xs">{r.action}</td>
                      <td className="px-3 py-2.5">
                        {r.entity_type ? (
                          <>
                            {t(`audit.entities.${r.entity_type}`, { defaultValue: r.entity_type })}
                            {r.entity_id && (
                              <span className="ml-1 font-mono text-xs text-ink-muted" title={r.entity_id}>
                                {shortId(r.entity_id)}
                              </span>
                            )}
                          </>
                        ) : (
                          '—'
                        )}
                      </td>
                      <td className="px-3 py-2.5">{r.reason ?? '—'}</td>
                      <td className="px-3 py-2.5 font-mono text-xs whitespace-nowrap">{r.ip_address ?? '—'}</td>
                    </tr>
                    {expanded && (
                      <tr className="border-b border-border">
                        <td id={`audit-${r.id}`} colSpan={6} className="bg-canvas/50 px-3 py-3">
                          <AuditDetails record={r} />
                        </td>
                      </tr>
                    )}
                  </Fragment>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
      {logs.hasNextPage && (
        <Button variant="secondary" className="self-center" disabled={logs.isFetchingNextPage} onClick={() => void logs.fetchNextPage()}>
          {logs.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
    </div>
  )
}
