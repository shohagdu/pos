import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'

import { ApiError } from '../../api/client'
import { type Customer, partiesApi, partyKeys, useCustomer } from '../../api/parties'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { PageHeader } from '../../components/ui/PageHeader'
import { StateTabs } from '../../components/ui/Tabs'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { LinkButton, QueryStatus } from '../money/shared'
import { useTenant } from '../money/utils'
import { Balance } from './Balance'
import { InvoicesTab, PaymentsTab, StatementTab } from './CustomerTabs'
import { ReceivePaymentDialog } from './ReceivePaymentDialog'
import { customerInput } from './utils'

type Tab = 'statement' | 'invoices' | 'payments'

/** One customer (`/customers/:id`): balance, actions, statement, invoices and payments. */
export function CustomerDetailPage() {
  const { id = '' } = useParams()
  const { tenantId } = useTenant()
  const customer = useCustomer(tenantId, id)
  if (!customer.data) return <QueryStatus query={customer} />
  return <CustomerDetail key={id} customer={customer.data} tenantId={tenantId} />
}

function CustomerDetail({ customer: c, tenantId }: { customer: Customer; tenantId: string }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { can } = useTenant()
  const [tab, setTab] = useState<Tab>('statement')
  const [receiving, setReceiving] = useState(false)
  const canManage = can('customers.manage') && !c.is_walk_in

  const refresh = () => queryClient.invalidateQueries({ queryKey: partyKeys.customers(tenantId) })
  const status = useMutation({
    mutationFn: () => partiesApi.updateCustomer(c.id, c.version, customerInput(c, { status: c.status === 'active' ? 'inactive' : 'active' })),
    onSuccess: async (saved) => {
      queryClient.setQueryData(partyKeys.customer(tenantId, saved.id), saved)
      await refresh()
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412) void refresh()
    },
  })
  const remove = useMutation({
    mutationFn: () => partiesApi.deleteCustomer(c.id),
    onSuccess: async () => {
      queryClient.removeQueries({ queryKey: partyKeys.customer(tenantId, c.id) })
      await refresh()
      navigate('/customers', { replace: true })
    },
  })
  const inUse = remove.error instanceof ApiError && remove.error.problem.code === 'IN_USE'

  return (
    <div className="flex flex-col gap-6">
      <Link to="/customers" className="text-sm font-medium text-brand-700 hover:underline print:hidden">
        ← {t('customers.title')}
      </Link>
      <PageHeader
        title={c.name}
        subtitle={
          <span className="inline-flex flex-wrap items-center gap-2">
            <span className="font-mono">{c.code}</span>
            {c.is_walk_in && <Badge tone="info">{t('customers.walkIn')}</Badge>}
            <Badge tone={c.status === 'active' ? 'good' : 'neutral'}>{t(`customers.statuses.${c.status}`)}</Badge>
          </span>
        }
        actions={
          <div className="flex flex-wrap items-center gap-2 print:hidden">
            {can('customers.collect_payment') && !c.is_walk_in && (
              <Button onClick={() => setReceiving(true)} disabled={c.status !== 'active'}>
                {t('customers.receivePayment')}
              </Button>
            )}
            {canManage && <LinkButton to={`/customers/${c.id}/edit`}>{t('common.edit')}</LinkButton>}
            {canManage && (
              <Button variant="secondary" disabled={status.isPending} onClick={() => status.mutate()}>
                {c.status === 'active' ? t('customers.deactivate') : t('customers.activate')}
              </Button>
            )}
            {canManage && (
              <ConfirmButton disabled={remove.isPending} onConfirm={() => remove.mutate()}>
                {t('common.delete')}
              </ConfirmButton>
            )}
          </div>
        }
      />
      {status.isError && <Alert tone="danger">{problemMessage(status.error, t)}</Alert>}
      {remove.isError && <Alert tone="danger">{inUse ? t('customers.inUse') : problemMessage(remove.error, t)}</Alert>}

      <Card>
        <dl className="grid gap-4 text-sm sm:grid-cols-2 lg:grid-cols-4">
          <div>
            <dt className="text-ink-muted">{t('customers.balance')}</dt>
            <dd className="mt-1 text-lg">
              <Balance value={c.current_balance} />
            </dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('customers.creditLimit')}</dt>
            <dd className="mt-1 text-lg font-semibold">{c.credit_limit === null ? t('customers.noLimit') : formatMoney(c.credit_limit, lang)}</dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('customers.phone')}</dt>
            <dd className="mt-1 font-medium">{c.phone ?? '—'}</dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('customers.email')}</dt>
            <dd className="mt-1 font-medium break-all">{c.email ?? '—'}</dd>
          </div>
          {c.address && (
            <div className="sm:col-span-2">
              <dt className="text-ink-muted">{t('customers.address')}</dt>
              <dd className="mt-1">{c.address}</dd>
            </div>
          )}
          {c.notes && (
            <div className="sm:col-span-2">
              <dt className="text-ink-muted">{t('customers.notes')}</dt>
              <dd className="mt-1 whitespace-pre-line">{c.notes}</dd>
            </div>
          )}
        </dl>
      </Card>

      <div className="print:hidden">
        <StateTabs<Tab>
          label={t('customers.sections')}
          value={tab}
          onChange={setTab}
          items={[
            { value: 'statement', label: t('customers.statement') },
            { value: 'invoices', label: t('customers.invoices') },
            { value: 'payments', label: t('customers.payments') },
          ]}
        />
      </div>
      {tab === 'statement' && <StatementTab customerId={c.id} customerName={c.name} />}
      {tab === 'invoices' && <InvoicesTab customerId={c.id} />}
      {tab === 'payments' && <PaymentsTab customerId={c.id} />}

      {receiving && <ReceivePaymentDialog customer={c} tenantId={tenantId} onClose={() => setReceiving(false)} />}
    </div>
  )
}
