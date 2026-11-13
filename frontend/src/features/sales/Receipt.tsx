import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

import type { ReceiptInfo, SaleDetail, SaleReturnDetail } from '../../api/sales'
import { ZERO, dec, toFixed } from '../../lib/decimal'
import { formatDate, formatDateTime } from '../../lib/dates'
import { formatMoney, formatQuantity } from '../../lib/format'
import { warrantyTenure } from './warranty'

// Printable receipt (Requirement Analysis §7.6.5). The width follows the
// receipt paper: 80 mm rolls print ~72 mm, 58 mm rolls ~48 mm.
const PAPER: Record<string, string> = {
  '80mm': 'w-[72mm] text-[11px]',
  '58mm': 'w-[48mm] text-[10px]',
  a4: 'w-full max-w-[190mm] text-sm',
}

const nonZero = (v: string | null | undefined) => dec(v) !== ZERO

function Row({ label, value, strong = false }: { label: ReactNode; value: ReactNode; strong?: boolean }) {
  return (
    <div className={`flex justify-between gap-2 ${strong ? 'text-[1.15em] font-bold' : ''}`}>
      <span>{label}</span>
      <span className="text-right tabular-nums whitespace-nowrap">{value}</span>
    </div>
  )
}

/** Header over the item lines: serial number, item, total. */
function ItemsHeader() {
  const { t } = useTranslation()
  return (
    <div className="flex gap-1 font-semibold">
      <span className="w-6 shrink-0">{t('sales.receipt.sl')}</span>
      <span className="flex-1">{t('sales.receipt.item')}</span>
      <span className="text-right">{t('sales.receipt.total')}</span>
    </div>
  )
}

/** One item line with its serial number hanging on the left. */
function NumberedItem({ n, children }: { n: number; children: ReactNode }) {
  return (
    <div className="mb-1 flex gap-1">
      <span className="w-6 shrink-0 tabular-nums">{n}.</span>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  )
}

function Rule() {
  return <hr className="my-1.5 border-0 border-t border-dashed border-ink" />
}

function ReceiptFrame({ info, children, marker }: { info: ReceiptInfo; children: ReactNode; marker?: ReactNode }) {
  const { t } = useTranslation()
  // Italian receipts print the Partita IVA, codice fiscale and R.E.A.
  const italy = info.country_code === 'IT'
  return (
    <div
      data-testid="receipt"
      className={`mx-auto bg-surface font-sans leading-snug text-ink print:bg-white print:text-black ${PAPER[info.receipt_paper] ?? PAPER['80mm']}`}
    >
      <div className="text-center">
        <div className="text-[1.3em] font-bold">{info.business_name}</div>
        {italy && info.legal_name && <div>{info.legal_name}</div>}
        {info.branch_name && <div>{info.branch_name}</div>}
        {info.address && <div>{info.address}</div>}
        {info.phone && <div>{t('sales.receipt.phone', { phone: info.phone })}</div>}
        {italy ? (
          <>
            {info.bin_number && <div>{t('sales.receipt.partitaIva', { value: info.bin_number })}</div>}
            {info.tax_code && <div>{t('sales.receipt.taxCode', { value: info.tax_code })}</div>}
            {info.rea_number && (
              <div>{t('sales.receipt.rea', { office: info.registry_office ?? '', number: info.rea_number }).replace(/\s+/g, ' ')}</div>
            )}
          </>
        ) : (
          info.bin_number && <div>{t('sales.receipt.bin', { bin: info.bin_number })}</div>
        )}
        {info.receipt_header && <div className="mt-1 whitespace-pre-line">{info.receipt_header}</div>}
        {marker && <div className="mt-1 font-bold">{marker}</div>}
      </div>
      <Rule />
      {children}
      {info.receipt_footer && (
        <>
          <Rule />
          <div className="text-center whitespace-pre-line">{info.receipt_footer}</div>
        </>
      )}
    </div>
  )
}

/** A sale receipt: POS success, sale detail (reprint). */
export function SaleReceipt({ sale }: { sale: SaleDetail }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const money = (v: string) => formatMoney(v, lang)
  const discount = dec(sale.item_discount_total) + dec(sale.invoice_discount_amount)
  const marker =
    sale.status === 'cancelled'
      ? t('sales.receipt.cancelled')
      : sale.reprint_count > 0
        ? t('sales.receipt.reprint', { n: sale.reprint_count })
        : null
  return (
    <ReceiptFrame info={sale.receipt} marker={marker}>
      <Row label={t('sales.receipt.invoice')} value={sale.invoice_no} />
      <Row label={t('sales.receipt.date')} value={formatDateTime(sale.sale_date || sale.created_at, lang)} />
      {sale.cashier_name && <Row label={t('sales.receipt.cashier')} value={sale.cashier_name} />}
      {!sale.is_walk_in && (
        <Row label={t('sales.receipt.customer')} value={[sale.customer_name, sale.customer_phone].filter(Boolean).join(' · ')} />
      )}
      <Rule />
      <ItemsHeader />
      {sale.items.map((item, i) => (
        <NumberedItem key={item.id} n={i + 1}>
          <div className="font-medium">{item.product_name}</div>
          <Row
            label={`${formatQuantity(item.quantity, lang, true)} ${item.unit_short_name} × ${money(item.unit_price)}`}
            value={money(item.line_total)}
          />
          {nonZero(item.line_discount) && <Row label={t('sales.receipt.lineDiscount')} value={`−${money(item.line_discount)}`} />}
          {item.warranty_expires_on && (
            <div>{t('sales.receipt.warranty', { tenure: warrantyTenure(item, t) ?? '', date: formatDate(item.warranty_expires_on, lang) })}</div>
          )}
        </NumberedItem>
      ))}
      <Rule />
      <Row label={t('sales.receipt.subtotal')} value={money(sale.subtotal)} />
      {discount !== ZERO && <Row label={t('sales.receipt.discount')} value={`−${money(toFixed(discount, 2))}`} />}
      {nonZero(sale.tax_total) && (
        <Row label={sale.prices_include_tax ? t('sales.receipt.vatIncluded') : t('sales.receipt.vat')} value={money(sale.tax_total)} />
      )}
      {nonZero(sale.shipping_charge) && <Row label={t('sales.receipt.shipping')} value={money(sale.shipping_charge)} />}
      {nonZero(sale.other_charge) && <Row label={t('sales.receipt.otherCharge')} value={money(sale.other_charge)} />}
      {nonZero(sale.rounding_adjustment) && <Row label={t('sales.receipt.rounding')} value={money(sale.rounding_adjustment)} />}
      <Row strong label={t('sales.receipt.grandTotal')} value={money(sale.grand_total)} />
      <Rule />
      {sale.payments
        .filter((p) => p.status === 'posted')
        .map((p) => (
          <Row key={p.payment_id} label={p.reference ? `${p.method_name} (${p.reference})` : p.method_name} value={money(p.amount)} />
        ))}
      <Row label={t('sales.receipt.paid')} value={money(sale.paid_total)} />
      {nonZero(sale.change_given) && <Row label={t('sales.receipt.change')} value={money(sale.change_given)} />}
      {nonZero(sale.due_total) && <Row strong label={t('sales.receipt.due')} value={money(sale.due_total)} />}
      {nonZero(sale.returned_total) && <Row label={t('sales.receipt.returned')} value={money(sale.returned_total)} />}
      {sale.note && <div className="mt-1 whitespace-pre-line">{sale.note}</div>}
    </ReceiptFrame>
  )
}

/** A sales return receipt. */
export function ReturnReceipt({ ret }: { ret: SaleReturnDetail }) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const money = (v: string) => formatMoney(v, lang)
  return (
    <ReceiptFrame info={ret.receipt} marker={t('sales.receipt.returnTitle')}>
      <Row label={t('sales.receipt.returnNo')} value={ret.return_no} />
      <Row label={t('sales.receipt.invoice')} value={ret.invoice_no} />
      <Row label={t('sales.receipt.date')} value={formatDateTime(ret.created_at, lang)} />
      {ret.created_by_name && <Row label={t('sales.receipt.cashier')} value={ret.created_by_name} />}
      <Row label={t('sales.receipt.customer')} value={ret.customer_name} />
      <Rule />
      <ItemsHeader />
      {ret.items.map((item, i) => (
        <NumberedItem key={item.id} n={i + 1}>
          <div className="font-medium">{item.product_name}</div>
          <Row
            label={`${formatQuantity(item.quantity, lang, true)} ${item.unit_short_name} · ${t(`sales.conditions.${item.item_condition}`)}`}
            value={money(item.line_total)}
          />
        </NumberedItem>
      ))}
      <Rule />
      {nonZero(ret.tax_total) && <Row label={t('sales.receipt.vat')} value={money(ret.tax_total)} />}
      <Row strong label={t('sales.receipt.returnTotal')} value={money(ret.total)} />
      {nonZero(ret.applied_to_due) && <Row label={t('sales.receipt.appliedToDue')} value={money(ret.applied_to_due)} />}
      {nonZero(ret.refunded_amount) && <Row label={t('sales.receipt.refunded')} value={money(ret.refunded_amount)} />}
      {nonZero(ret.credited_amount) && <Row label={t('sales.receipt.credited')} value={money(ret.credited_amount)} />}
      <div className="mt-1">{t('sales.receipt.reason', { reason: ret.reason })}</div>
    </ReceiptFrame>
  )
}
