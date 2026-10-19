import { useTranslation } from 'react-i18next'

import type { SaleDetail } from '../../api/sales'
import { Card } from '../../components/ui/Card'
import { ZERO, dec } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'

/** The totals block of a sale detail. */
export function SaleTotals({ sale }: { sale: SaleDetail }) {
  const { t, i18n } = useTranslation()
  const money = (v: string) => formatMoney(v, i18n.language)
  const rows: { key: string; label: string; value: string; always?: boolean; strong?: boolean; negative?: boolean }[] = [
    { key: 'subtotal', label: t('sales.subtotal'), value: sale.subtotal, always: true },
    { key: 'itemDiscount', label: t('sales.itemDiscounts'), value: sale.item_discount_total, negative: true },
    { key: 'invoiceDiscount', label: t('sales.invoiceDiscount'), value: sale.invoice_discount_amount, negative: true },
    { key: 'vat', label: sale.prices_include_tax ? t('sales.vatIncluded') : t('sales.vat'), value: sale.tax_total },
    { key: 'shipping', label: t('sales.shipping'), value: sale.shipping_charge },
    { key: 'other', label: t('sales.otherCharge'), value: sale.other_charge },
    { key: 'rounding', label: t('sales.rounding'), value: sale.rounding_adjustment },
    { key: 'grand', label: t('sales.grandTotal'), value: sale.grand_total, always: true, strong: true },
    { key: 'paid', label: t('sales.paid'), value: sale.paid_total, always: true },
    { key: 'change', label: t('sales.change'), value: sale.change_given },
    { key: 'returned', label: t('sales.returned'), value: sale.returned_total },
    { key: 'due', label: t('sales.due'), value: sale.due_total, always: true, strong: true },
  ]
  if (sale.cost_total !== null) rows.push({ key: 'cost', label: t('sales.costTotal'), value: sale.cost_total })
  return (
    <Card className="self-start p-4">
      <h2 className="mb-2 text-lg font-semibold">{t('common.totals')}</h2>
      <dl className="flex flex-col gap-1 text-sm">
        {rows
          .filter((r) => r.always || dec(r.value) !== ZERO)
          .map((r) => (
            <div key={r.key} className={`flex justify-between gap-3 ${r.strong ? 'border-t border-border pt-1 text-base font-bold' : ''}`}>
              <dt>{r.label}</dt>
              <dd className="tabular-nums">
                {r.negative ? '−' : ''}
                {money(r.value)}
              </dd>
            </div>
          ))}
      </dl>
    </Card>
  )
}
