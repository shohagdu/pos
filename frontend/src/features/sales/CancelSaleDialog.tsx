import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type SaleDetail, salesApi, salesKeys } from '../../api/sales'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { fieldErrors, problemMessage } from '../../lib/problem'

/** Cancels a same-day sale (stock back, payments voided) with a reason. */
export function CancelSaleDialog({ sale, tenantId, open, onClose }: { sale: SaleDetail; tenantId: string; open: boolean; onClose: () => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [reason, setReason] = useState('')
  const [missing, setMissing] = useState(false)

  const cancel = useMutation({
    mutationFn: () => salesApi.cancel(sale.id, reason.trim()),
    onSuccess: (detail) => {
      queryClient.setQueryData(salesKeys.sale(tenantId, sale.id), detail)
      for (const key of ['sales', 'inventory', 'parties', 'money', 'reports']) void queryClient.invalidateQueries({ queryKey: [key, tenantId] })
      onClose()
    },
  })

  function submit(e: FormEvent) {
    e.preventDefault()
    if (!reason.trim()) {
      setMissing(true)
      return
    }
    cancel.mutate()
  }

  const errors = fieldErrors(cancel.error, t)
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={t('sales.cancelTitle', { no: sale.invoice_no })}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.close')}
          </Button>
          <Button type="submit" form="cancel-sale" className="bg-danger hover:bg-danger/90" disabled={cancel.isPending}>
            {cancel.isPending ? t('common.saving') : t('sales.cancelSale')}
          </Button>
        </>
      }
    >
      <form id="cancel-sale" onSubmit={submit} className="flex flex-col gap-3">
        <p className="text-sm text-ink-muted">{t('sales.cancelHint')}</p>
        {cancel.isError && !errors.reason && <Alert tone="danger">{problemMessage(cancel.error, t)}</Alert>}
        <TextField
          label={t('common.reason')}
          value={reason}
          maxLength={500}
          onChange={(e) => {
            setReason(e.target.value)
            setMissing(false)
          }}
          error={missing ? t('sales.reasonRequired') : errors.reason}
          autoFocus
          data-autofocus
        />
      </form>
    </Dialog>
  )
}
