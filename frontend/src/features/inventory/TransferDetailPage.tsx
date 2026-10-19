import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'

import { type Transfer, type TransferItem, inventoryApi, inventoryKeys, useTransfer } from '../../api/inventory'
import { newKey } from '../../api/query'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { Dialog } from '../../components/ui/Dialog'
import { PageHeader } from '../../components/ui/PageHeader'
import { TextField } from '../../components/ui/TextField'
import { dec, mul, toFixed, toPlain, tryParse } from '../../lib/decimal'
import { formatDate, formatDateTime } from '../../lib/dates'
import { formatMoney, formatQuantity } from '../../lib/format'
import { optional } from '../../lib/form'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { QueryStatus } from '../money/shared'
import { useTenant } from '../money/utils'
import { TRANSFER_TONES } from './tones'

const INPUT =
  'min-h-10 w-full rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100 aria-invalid:border-danger'

/** One stock transfer with dispatch, receive and cancel (`/inventory/transfers/:id`). */
export function TransferDetailPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { id = '' } = useParams()
  const queryClient = useQueryClient()
  const { tenantId, can } = useTenant()
  const transfer = useTransfer(tenantId, id)
  const [receiving, setReceiving] = useState(false)
  const [dispatchKey, setDispatchKey] = useState(newKey)

  const refresh = async (saved: Transfer) => {
    queryClient.setQueryData(inventoryKeys.transfer(tenantId, saved.id), saved)
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: inventoryKeys.all(tenantId) }),
      queryClient.invalidateQueries({ queryKey: ['reports', tenantId] }),
    ])
  }
  const dispatch = useMutation({
    mutationFn: () => inventoryApi.dispatchTransfer(id, dispatchKey),
    onSuccess: async (saved) => {
      setDispatchKey(newKey())
      await refresh(saved)
    },
  })
  const cancel = useMutation({ mutationFn: () => inventoryApi.cancelTransfer(id), onSuccess: refresh })

  const x = transfer.data
  if (!x) return <QueryStatus query={transfer} />

  const hasCost = x.items.some((i) => i.unit_cost !== null)
  const arrived = x.status === 'received'
  const loss = x.loss_value !== null && dec(x.loss_value) > 0n
  const actionError = dispatch.error ?? cancel.error
  const columns: Column<TransferItem>[] = [
    {
      key: 'product',
      header: t('inventory.product'),
      render: (i) => (
        <div className="flex flex-col">
          <span className="font-medium">{i.product_name}</span>
          <span className="font-mono text-xs text-ink-muted">{i.sku}</span>
        </div>
      ),
    },
    { key: 'sent', header: t('inventory.sent'), align: 'right', render: (i) => `${formatQuantity(i.quantity_sent, lang, true)} ${i.unit_short_name}` },
    ...(arrived
      ? ([
          {
            key: 'received',
            header: t('inventory.receivedQty'),
            align: 'right',
            render: (i) => {
              const short = i.quantity_received !== null && dec(i.quantity_received) < dec(i.quantity_sent)
              return (
                <span className={short ? 'text-danger' : ''}>
                  {i.quantity_received === null ? '—' : `${formatQuantity(i.quantity_received, lang, true)} ${i.unit_short_name}`}
                </span>
              )
            },
          },
        ] satisfies Column<TransferItem>[])
      : []),
    ...(hasCost
      ? ([
          { key: 'cost', header: t('inventory.unitCost'), align: 'right', render: (i) => (i.unit_cost === null ? '—' : formatMoney(i.unit_cost, lang)) },
          {
            key: 'value',
            header: t('inventory.value'),
            align: 'right',
            render: (i) => (i.unit_cost === null ? '—' : formatMoney(toFixed(mul(dec(i.quantity_sent), dec(i.unit_cost))), lang)),
          },
        ] satisfies Column<TransferItem>[])
      : []),
  ]

  const title = `${t('inventory.transfer')} ${x.transfer_no ?? ''}`.trim()
  return (
    <div className="flex max-w-4xl flex-col gap-6">
      <Link to="/inventory/transfers" className="text-sm font-medium text-brand-700 hover:underline print:hidden">
        ← {t('inventory.transfers')}
      </Link>
      <PageHeader
        title={title}
        subtitle={formatDate(x.business_date, lang)}
        actions={
          <div className="flex flex-wrap items-center gap-2 print:hidden">
            {x.status === 'draft' && can('inventory.transfer') && (
              <>
                <Button disabled={dispatch.isPending} onClick={() => dispatch.mutate()}>
                  {dispatch.isPending ? t('common.saving') : t('inventory.dispatch')}
                </Button>
                <ConfirmButton disabled={cancel.isPending} onConfirm={() => cancel.mutate()}>
                  {t('inventory.cancelTransfer')}
                </ConfirmButton>
              </>
            )}
            {x.status === 'dispatched' && can('inventory.transfer_receive') && <Button onClick={() => setReceiving(true)}>{t('inventory.receive')}</Button>}
            <Button variant="secondary" onClick={() => window.print()}>
              {t('common.print')}
            </Button>
          </div>
        }
      />
      {actionError && <Alert tone="danger">{problemMessage(actionError, t)}</Alert>}
      <Card>
        <dl className="grid gap-4 text-sm sm:grid-cols-3">
          <div>
            <dt className="text-ink-muted">{t('common.status')}</dt>
            <dd>
              <Badge tone={TRANSFER_TONES[x.status]}>{t(`inventory.transferStatuses.${x.status}`, { defaultValue: x.status })}</Badge>
            </dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('inventory.fromLocation')}</dt>
            <dd className="font-medium">
              {x.from_branch_name} — {x.from_location_name}
            </dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('inventory.toLocation')}</dt>
            <dd className="font-medium">
              {x.to_branch_name} — {x.to_location_name}
            </dd>
          </div>
          <div>
            <dt className="text-ink-muted">{t('inventory.itemCount')}</dt>
            <dd className="font-medium">{x.item_count}</dd>
          </div>
          {x.total_value !== null && x.status !== 'draft' && (
            <div>
              <dt className="text-ink-muted">{t('inventory.value')}</dt>
              <dd className="font-medium">{formatMoney(x.total_value, lang)}</dd>
            </div>
          )}
          {loss && (
            <div>
              <dt className="text-ink-muted">{t('inventory.transitLoss')}</dt>
              <dd className="font-medium text-danger">{formatMoney(x.loss_value ?? '0', lang)}</dd>
            </div>
          )}
          <div>
            <dt className="text-ink-muted">{t('inventory.createdBy')}</dt>
            <dd className="font-medium">
              {x.created_by_name ?? '—'} · {formatDateTime(x.created_at, lang)}
            </dd>
          </div>
          {x.dispatched_at && (
            <div>
              <dt className="text-ink-muted">{t('inventory.dispatchedAt')}</dt>
              <dd className="font-medium">{formatDateTime(x.dispatched_at, lang)}</dd>
            </div>
          )}
          {x.received_at && (
            <div>
              <dt className="text-ink-muted">{t('inventory.receivedAt')}</dt>
              <dd className="font-medium">{formatDateTime(x.received_at, lang)}</dd>
            </div>
          )}
          {x.note && (
            <div className="sm:col-span-3">
              <dt className="text-ink-muted">{t('common.note')}</dt>
              <dd className="whitespace-pre-line">{x.note}</dd>
            </div>
          )}
        </dl>
      </Card>
      <DataTable caption={t('inventory.lines')} columns={columns} rows={x.items} rowKey={(i) => i.id} />
      {receiving && <ReceiveDialog transfer={x} onDone={refresh} onClose={() => setReceiving(false)} />}
    </div>
  )
}

/** Receives a dispatched transfer; only lines that arrived short are sent. */
function ReceiveDialog({ transfer, onDone, onClose }: { transfer: Transfer; onDone: (saved: Transfer) => Promise<void>; onClose: () => void }) {
  const { t, i18n } = useTranslation()
  const [received, setReceived] = useState<Record<string, string>>(() =>
    Object.fromEntries(transfer.items.map((i) => [i.id, toPlain(dec(i.quantity_sent))])),
  )
  const [note, setNote] = useState('')
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})
  const [key] = useState(newKey)

  const save = useMutation({
    mutationFn: () => {
      const lines = transfer.items
        .filter((i) => tryParse(received[i.id]) !== dec(i.quantity_sent))
        .map((i) => ({ item_id: i.id, quantity_received: (received[i.id] ?? '').trim() }))
      return inventoryApi.receiveTransfer(transfer.id, { lines: lines.length ? lines : undefined, note: optional(note) }, key)
    },
    onSuccess: async (saved) => {
      await onDone(saved)
      onClose()
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errs: Record<string, string> = {}
    transfer.items.forEach((i) => {
      const q = tryParse(received[i.id])
      if (q === null || q < 0n || q > dec(i.quantity_sent)) errs[i.id] = t('inventory.errors.received')
    })
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) save.mutate()
  }

  const serverErrors = fieldErrors(save.error, t)
  return (
    <Dialog open onClose={onClose} title={t('inventory.receiveTitle', { no: transfer.transfer_no ?? '' })} wide>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        <p className="text-sm text-ink-muted">{t('inventory.receiveHint')}</p>
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <ul className="flex flex-col divide-y divide-border rounded-lg border border-border">
          {transfer.items.map((i, n) => {
            const err = localErrors[i.id] ?? serverErrors[`lines[${n}].quantity_received`]
            return (
              <li key={i.id} className="flex flex-wrap items-center justify-between gap-2 px-3 py-2 text-sm">
                <div className="flex flex-col">
                  <span className="font-medium">{i.product_name}</span>
                  <span className="text-xs text-ink-muted">
                    {t('inventory.sent')}: {formatQuantity(i.quantity_sent, i18n.language, true)} {i.unit_short_name}
                  </span>
                </div>
                <div className="flex flex-col items-end">
                  <div className="flex items-center gap-2">
                    <input
                      aria-label={t('inventory.receivedFor', { name: i.product_name })}
                      aria-invalid={err ? true : undefined}
                      className={`${INPUT} max-w-28 text-right`}
                      inputMode="decimal"
                      value={received[i.id] ?? ''}
                      onChange={(e) => setReceived((r) => ({ ...r, [i.id]: e.target.value }))}
                    />
                    <span className="text-ink-muted">{i.unit_short_name}</span>
                  </div>
                  {err && <p className="text-sm text-danger">{err}</p>}
                </div>
              </li>
            )
          })}
        </ul>
        <TextField label={`${t('common.note')} (${t('common.optional')})`} value={note} onChange={(e) => setNote(e.target.value)} maxLength={2000} />
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t('common.saving') : t('inventory.confirmReceive')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
