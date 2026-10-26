import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import type { SaleDetail } from '../../api/sales'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { ZERO, dec } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { SaleReceipt } from '../sales/Receipt'

/** After a sale: the receipt, Print, and "New sale". */
export function ReceiptDialog({ sale, onPrint, onNewSale }: { sale: SaleDetail; onPrint: () => void; onNewSale: () => void }) {
  const { t, i18n } = useTranslation()
  const printed = useRef<string | null>(null)

  // Auto-print once per sale when the business asks for it.
  useEffect(() => {
    if (sale.receipt.auto_print && printed.current !== sale.id) {
      printed.current = sale.id
      onPrint()
    }
  }, [sale, onPrint])

  return (
    <Dialog
      open
      onClose={onNewSale}
      title={t('pos.saleDone', { no: sale.invoice_no })}
      footer={
        <>
          <Button variant="secondary" onClick={onPrint}>
            {t('common.print')}
          </Button>
          <Button onClick={onNewSale} autoFocus data-autofocus>
            {t('pos.newSale')}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-3">
        {dec(sale.change_given) > ZERO && (
          <p className="rounded-lg bg-brand-50 px-4 py-3 text-center text-lg font-bold text-brand-700">
            {t('pos.giveChange', { amount: formatMoney(sale.change_given, i18n.language) })}
          </p>
        )}
        <div className="rounded-lg border border-border p-3">
          <SaleReceipt sale={sale} />
        </div>
      </div>
    </Dialog>
  )
}
