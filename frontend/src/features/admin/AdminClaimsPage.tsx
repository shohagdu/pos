import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import type { PaymentClaim } from '../../api/billing'
import { canBill, platformApi, platformKeys, useAdminClaims } from '../../api/platform'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { PageHeader } from '../../components/ui/PageHeader'
import { Spinner } from '../../components/ui/Spinner'
import { StateTabs } from '../../components/ui/Tabs'
import { formatDateTime } from '../../lib/dates'
import { dec } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { LINK, cycleLabel, methodLabel, useStaffRole } from './labels'
import { ClaimStatusBadge, ReasonDialog } from './shared'

const TABS = ['pending', 'approved', 'rejected', 'all'] as const
type Tab = (typeof TABS)[number]

/** Manual payment claims (SUB-R05, ADM-R04): review and approve or reject. */
export function AdminClaimsPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const role = useStaffRole()
  const queryClient = useQueryClient()
  const [tab, setTab] = useState<Tab>('pending')
  const [rejecting, setRejecting] = useState<PaymentClaim | null>(null)
  const claims = useAdminClaims(tab)

  const refresh = () => queryClient.invalidateQueries({ queryKey: platformKeys.all })
  const approve = useMutation({ mutationFn: (id: string) => platformApi.approveClaim(id), onSuccess: refresh })
  const reject = useMutation({
    mutationFn: ({ id, reason }: { id: string; reason: string }) => platformApi.rejectClaim(id, reason),
    onSuccess: async () => {
      setRejecting(null)
      await refresh()
    },
  })

  const rows = claims.data?.pages.flatMap((p) => p.data) ?? []
  const pendingCount = claims.data?.pages[0]?.pending_count
  const decide = canBill(role)

  const columns: Column<PaymentClaim>[] = [
    { key: 'submitted', header: t('admin.claims.submitted'), className: 'whitespace-nowrap', render: (c) => formatDateTime(c.created_at, lang) },
    {
      key: 'business',
      header: t('admin.claims.business'),
      render: (c) => (
        <Link to={`/admin/tenants/${c.tenant_id}`} className={LINK}>
          {c.tenant_name}
        </Link>
      ),
    },
    { key: 'invoice', header: t('admin.claims.invoice'), render: (c) => <span className="font-mono text-xs">{c.invoice_no}</span> },
    { key: 'plan', header: t('admin.tenants.plan'), render: (c) => `${c.plan_name} · ${cycleLabel(t, c.billing_cycle)}` },
    { key: 'invoiceTotal', header: t('admin.claims.invoiceTotal'), align: 'right', render: (c) => formatMoney(c.invoice_total, lang) },
    { key: 'method', header: t('admin.claims.method'), render: (c) => <Badge tone="info">{methodLabel(t, c.method)}</Badge> },
    {
      key: 'amount',
      header: t('common.amount'),
      align: 'right',
      render: (c) => {
        const differs = dec(c.amount) !== dec(c.invoice_total)
        return (
          <span className={differs ? 'font-semibold text-danger' : ''}>
            {formatMoney(c.amount, lang)}
            {differs && <span className="block text-xs font-normal">{t('admin.claims.differs')}</span>}
          </span>
        )
      },
    },
    { key: 'ref', header: t('admin.claims.reference'), render: (c) => <span className="font-mono text-xs">{c.transaction_ref}</span> },
    { key: 'payer', header: t('admin.claims.payer'), render: (c) => c.payer_account ?? '—' },
    { key: 'by', header: t('admin.claims.submittedBy'), render: (c) => c.submitted_by_name ?? '—' },
    { key: 'status', header: t('common.status'), render: (c) => <ClaimStatusBadge status={c.status} /> },
    {
      key: 'reviewed',
      header: t('admin.claims.reviewed'),
      render: (c) =>
        c.reviewed_at ? (
          <>
            {c.reviewed_by_name ?? '—'}
            <span className="block text-xs whitespace-nowrap text-ink-muted">{formatDateTime(c.reviewed_at, lang)}</span>
          </>
        ) : (
          '—'
        ),
    },
    { key: 'rejection', header: t('admin.claims.rejectionReason'), render: (c) => c.rejection_reason ?? '—' },
  ]
  if (decide && rows.some((c) => c.status === 'pending')) {
    columns.push({
      key: 'actions',
      header: t('common.actions'),
      render: (c) =>
        c.status === 'pending' && (
          <span className="flex flex-wrap gap-2">
            <ConfirmButton disabled={approve.isPending} onConfirm={() => approve.mutate(c.id)}>
              {t('admin.claims.approve')}
            </ConfirmButton>
            <Button
              variant="ghost"
              className="text-danger"
              onClick={() => {
                reject.reset()
                setRejecting(c)
              }}
            >
              {t('admin.claims.reject')}
            </Button>
          </span>
        ),
    })
  }

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t('admin.claims.title')}
        subtitle={pendingCount !== undefined ? t('admin.claims.pendingCount', { count: pendingCount }) : undefined}
      />
      <StateTabs<Tab>
        label={t('admin.claims.title')}
        value={tab}
        onChange={setTab}
        items={TABS.map((s) => ({ value: s, label: t(`admin.claims.tabs.${s}`) }))}
      />
      {approve.isError && <Alert tone="danger">{problemMessage(approve.error, t)}</Alert>}
      {approve.isSuccess && <Alert tone="success">{t('admin.claims.approved')}</Alert>}
      {claims.isPending ? (
        <Spinner />
      ) : claims.isError ? (
        <Alert tone="danger">{problemMessage(claims.error, t)}</Alert>
      ) : (
        <DataTable<PaymentClaim> caption={t('admin.claims.title')} rows={rows} rowKey={(c) => c.id} empty={t('admin.claims.none')} columns={columns} />
      )}
      {claims.hasNextPage && (
        <Button variant="secondary" className="self-center" disabled={claims.isFetchingNextPage} onClick={() => void claims.fetchNextPage()}>
          {claims.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
        </Button>
      )}
      <ReasonDialog
        open={rejecting !== null}
        onClose={() => setRejecting(null)}
        title={t('admin.claims.rejectTitle', { invoice: rejecting?.invoice_no ?? '' })}
        hint={t('admin.claims.rejectHint')}
        confirmLabel={t('admin.claims.reject')}
        pending={reject.isPending}
        error={reject.error}
        onSubmit={(reason) => rejecting && reject.mutate({ id: rejecting.id, reason })}
      />
    </div>
  )
}
