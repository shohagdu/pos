import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'

import { type DataArchive, type DocumentCounts, tenancyApi, tenancyKeys } from '../../api/tenancy'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Card } from '../../components/ui/Card'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { DataTable } from '../../components/ui/DataTable'
import { Spinner } from '../../components/ui/Spinner'
import { formatDateTime } from '../../lib/dates'
import { problemMessage } from '../../lib/problem'

/** "Sales 12 · Purchases 3 · …", or a dash. */
function useDocumentList() {
  const { t } = useTranslation()
  return (docs: DocumentCounts) => {
    const parts = Object.entries(docs)
      .filter(([, n]) => (n ?? 0) > 0)
      .map(([key, n]) => `${t(`settings.dataReset.docs.${key}`, { defaultValue: key })} ${n}`)
    return parts.length > 0 ? parts.join(' · ') : '—'
  }
}

/**
 * Owner only: clear every transaction with one click, and restore any archive.
 * Nothing is deleted for good: a clear is archived, and a restore archives
 * what is live first.
 */
export function DataResetSection({ tenantId }: { tenantId: string }) {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const list = useDocumentList()
  const overview = useQuery({ queryKey: tenancyKeys.dataReset(tenantId), queryFn: tenancyApi.dataReset })
  // Every list, report and balance changes.
  const refresh = () => queryClient.invalidateQueries()
  const clear = useMutation({ mutationFn: tenancyApi.clearTransactions, onSuccess: refresh })
  const restore = useMutation({ mutationFn: tenancyApi.restoreArchive, onSuccess: refresh })
  const busy = clear.isPending || restore.isPending
  const run = (action: () => void) => {
    clear.reset()
    restore.reset()
    action()
  }

  if (overview.isPending) return <Spinner />
  if (overview.isError) return <Alert tone="danger">{problemMessage(overview.error, t)}</Alert>
  const { live, archives } = overview.data
  const empty = Object.keys(live).length === 0

  return (
    <div className="flex flex-col gap-6">
      <Card className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">{t('settings.dataReset.clearTitle')}</h2>
        <p className="text-sm text-ink-muted">{t('settings.dataReset.clearIntro')}</p>
        <ul className="list-disc pl-5 text-sm text-ink-muted">
          <li>{t('settings.dataReset.clears')}</li>
          <li>{t('settings.dataReset.keeps')}</li>
          <li>{t('settings.dataReset.resets')}</li>
        </ul>
        <p className="text-sm">
          <span className="font-medium">{t('settings.dataReset.live')}:</span> {empty ? t('settings.dataReset.none') : list(live)}
        </p>
        <div>
          <ConfirmButton className="text-danger" disabled={busy || empty} onConfirm={() => run(() => clear.mutate())}>
            {clear.isPending ? t('settings.dataReset.clearing') : t('settings.dataReset.clear')}
          </ConfirmButton>
        </div>
        {clear.isSuccess && <Alert tone="success">{t('settings.dataReset.cleared')}</Alert>}
        {clear.isError && <Alert tone="danger">{problemMessage(clear.error, t)}</Alert>}
      </Card>

      <Card className="flex flex-col gap-3">
        <h2 className="text-lg font-semibold">{t('settings.dataReset.archivesTitle')}</h2>
        <p className="text-sm text-ink-muted">{t('settings.dataReset.restoreHint')}</p>
        {restore.isSuccess && <Alert tone="success">{t('settings.dataReset.restored')}</Alert>}
        {restore.isError && <Alert tone="danger">{problemMessage(restore.error, t)}</Alert>}
        <DataTable<DataArchive>
          caption={t('settings.dataReset.archivesTitle')}
          rows={archives}
          rowKey={(a) => a.id}
          empty={t('settings.dataReset.noArchives')}
          columns={[
            { key: 'when', header: t('settings.dataReset.when'), render: (a) => formatDateTime(a.created_at, i18n.language) },
            {
              key: 'kind',
              header: t('settings.dataReset.kind'),
              render: (a) => (
                <span className="flex flex-col">
                  <span>{t(`settings.dataReset.kinds.${a.kind}`)}</span>
                  {a.created_by_name && <span className="text-xs text-ink-muted">{a.created_by_name}</span>}
                </span>
              ),
            },
            { key: 'documents', header: t('settings.dataReset.documents'), render: (a) => list(a.documents) },
            {
              key: 'status',
              header: t('common.status'),
              render: (a) =>
                a.status === 'held' ? (
                  <Badge tone="warn">{t('settings.dataReset.statuses.held')}</Badge>
                ) : (
                  <Badge tone="good">
                    {t('settings.dataReset.statuses.restored')}
                    {a.restored_at ? ` · ${formatDateTime(a.restored_at, i18n.language)}` : ''}
                  </Badge>
                ),
            },
            {
              key: 'action',
              header: <span className="sr-only">{t('common.actions')}</span>,
              align: 'right' as const,
              render: (a) =>
                a.status === 'held' ? (
                  <ConfirmButton disabled={busy} onConfirm={() => run(() => restore.mutate(a.id))}>
                    {t('settings.dataReset.restore')}
                  </ConfirmButton>
                ) : null,
            },
          ]}
        />
      </Card>
    </div>
  )
}
