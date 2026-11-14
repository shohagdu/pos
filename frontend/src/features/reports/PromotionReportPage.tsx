import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type CouponUseRow, useReport } from '../../api/reports'
import { DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { StatCard } from '../../components/ui/StatCard'
import { downloadCsv } from '../../lib/csv'
import { formatDateTime } from '../../lib/dates'
import { dec, toPlain } from '../../lib/decimal'
import { formatQuantity } from '../../lib/format'
import { localDateTime } from '../promotions/format'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { money, usePeriodLabel, useRange } from './format'
import { type ExportAction, Loaded, ReportFrame, Section } from './shared'

/**
 * Promotions report (Final_docs/05 §10): performance and profit per
 * promotion, flash sales against their limit, coupon usage, and manual vs
 * automatic discounts per cashier.
 */
export function PromotionReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).promotions
  const [range, setRange] = useRange()
  const periodLabel = usePeriodLabel()
  const report = useReport(tenantId, 'promotions', range, allowed)
  const r = report.data
  const cashier = (name: string | null) => name ?? t('reports.discounts.unknownCashier')

  const exports: ExportAction[] = []
  if (r) {
    const file = (base: string) => `${base}-${range.from}-${range.to}`
    exports.push(
      {
        label: t('reports.promotions.byPromotion'),
        run: () =>
          downloadCsv(file('promotions'), r.promotions, [
            { header: t('reports.promotions.promotion'), value: (p) => p.name },
            { header: t('reports.promotions.type'), value: (p) => t(`promotions.types.${p.promotion_type}`) },
            { header: t('reports.promotions.sales'), value: (p) => p.sales },
            { header: t('reports.promotions.units'), value: (p) => p.units },
            { header: t('reports.promotions.revenue'), value: (p) => p.revenue },
            { header: t('reports.promotions.discount'), value: (p) => p.discount },
            { header: t('reports.promotions.cost'), value: (p) => p.cost },
            { header: t('reports.promotions.profit'), value: (p) => p.profit },
            { header: t('reports.promotions.basket'), value: (p) => p.average_basket },
          ]),
      },
      {
        label: t('reports.promotions.byCashier'),
        run: () =>
          downloadCsv(file('discounts-manual-vs-automatic'), r.cashiers, [
            { header: t('reports.discounts.cashier'), value: (c) => cashier(c.cashier_name) },
            { header: t('reports.discounts.invoices'), value: (c) => c.invoices },
            { header: t('reports.discounts.gross'), value: (c) => c.gross },
            { header: t('reports.promotions.manual'), value: (c) => c.manual },
            { header: t('reports.promotions.automatic'), value: (c) => c.automatic },
            { header: t('reports.promotions.coupon'), value: (c) => c.coupon },
          ]),
      },
    )
  }

  const couponTable = (title: string, rows: CouponUseRow[], label: string) => (
    <DataTable
      caption={title}
      rows={rows}
      rowKey={(c) => c.label}
      empty={t('reports.noData')}
      columns={[
        { key: 'label', header: label, render: (c) => c.label },
        { key: 'uses', header: t('reports.promotions.uses'), align: 'right', render: (c) => c.uses },
        { key: 'discount', header: t('reports.promotions.discount'), align: 'right', render: (c) => money(c.discount, lang) },
      ]}
    />
  )

  return (
    <ReportFrame
      title={t('reports.promotions.title')}
      allowed={allowed}
      period={periodLabel(range.from, range.to)}
      exports={exports}
      updatedAt={report.dataUpdatedAt}
      filters={<DateRangeFilter from={range.from} to={range.to} onChange={setRange} />}
    >
      <Loaded query={report}>
        {(d) => (
          <>
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
              <StatCard label={t('reports.discounts.gross')} value={money(d.gross, lang)} />
              <StatCard label={t('reports.promotions.automatic')} value={money(d.automatic, lang)} tone="bad" />
              <StatCard label={t('reports.promotions.coupon')} value={money(d.coupon, lang)} tone="bad" />
              <StatCard label={t('reports.promotions.manual')} value={money(d.manual, lang)} />
            </div>
            <Section title={t('reports.promotions.byPromotion')}>
              <p className="text-xs text-ink-muted">{t('reports.promotions.byPromotionHint')}</p>
              <DataTable
                caption={t('reports.promotions.byPromotion')}
                rows={d.promotions}
                rowKey={(p) => p.promotion_id}
                empty={t('reports.noData')}
                columns={[
                  {
                    key: 'name',
                    header: t('reports.promotions.promotion'),
                    render: (p) => (
                      <>
                        {can('promotions.view') ? (
                          <Link to={`/promotions/${p.promotion_id}`} className="font-medium text-brand-700 hover:underline">
                            {p.name}
                          </Link>
                        ) : (
                          <span className="font-medium">{p.name}</span>
                        )}
                        <span className="block text-xs text-ink-muted">{t(`promotions.types.${p.promotion_type}`)}</span>
                      </>
                    ),
                  },
                  { key: 'sales', header: t('reports.promotions.sales'), align: 'right', render: (p) => p.sales },
                  { key: 'units', header: t('reports.promotions.units'), align: 'right', render: (p) => formatQuantity(p.units, lang, true) },
                  { key: 'revenue', header: t('reports.promotions.revenue'), align: 'right', render: (p) => money(p.revenue, lang) },
                  { key: 'discount', header: t('reports.promotions.discount'), align: 'right', render: (p) => money(p.discount, lang) },
                  { key: 'cost', header: t('reports.promotions.cost'), align: 'right', showFrom: 'xl', render: (p) => money(p.cost, lang) },
                  {
                    key: 'profit',
                    header: t('reports.promotions.profit'),
                    align: 'right',
                    card: 'amount',
                    render: (p) => <span className={dec(p.profit) < 0n ? 'font-semibold text-danger' : 'font-semibold'}>{money(p.profit, lang)}</span>,
                  },
                  { key: 'basket', header: t('reports.promotions.basket'), align: 'right', showFrom: 'xl', render: (p) => money(p.average_basket, lang) },
                ]}
              />
            </Section>
            {d.flash_sales.length > 0 && (
              <Section title={t('reports.promotions.flash')}>
                <DataTable
                  caption={t('reports.promotions.flash')}
                  rows={d.flash_sales}
                  rowKey={(f) => f.promotion_id}
                  columns={[
                    { key: 'name', header: t('reports.promotions.promotion'), render: (f) => f.name },
                    {
                      key: 'window',
                      header: t('promotions.schedule'),
                      render: (f) => `${localDateTime(f.starts_at, lang)} → ${localDateTime(f.ends_at, lang)}`,
                    },
                    {
                      key: 'sold',
                      header: t('reports.promotions.soldOfLimit'),
                      align: 'right',
                      render: (f) =>
                        f.quantity_limit ? `${toPlain(dec(f.quantity_used))} / ${toPlain(dec(f.quantity_limit))}` : toPlain(dec(f.quantity_used)),
                    },
                    {
                      key: 'soldOut',
                      header: t('reports.promotions.soldOutAt'),
                      render: (f) => (f.sold_out_at ? formatDateTime(f.sold_out_at, lang) : t('reports.promotions.notSoldOut')),
                    },
                  ]}
                />
              </Section>
            )}
            <Section title={t('reports.promotions.coupons')}>
              <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
                {couponTable(t('reports.promotions.byCode'), d.coupon_codes, t('promotions.coupons.code'))}
                {couponTable(t('reports.promotions.byCustomer'), d.coupon_customers, t('promotions.coupons.customer'))}
                {couponTable(t('reports.promotions.byCashierCoupon'), d.coupon_cashiers, t('promotions.coupons.cashier'))}
              </div>
            </Section>
            <Section title={t('reports.promotions.byCashier')}>
              <p className="text-xs text-ink-muted">{t('reports.promotions.byCashierHint')}</p>
              <DataTable
                caption={t('reports.promotions.byCashier')}
                rows={d.cashiers}
                rowKey={(c) => c.cashier_id}
                empty={t('reports.noData')}
                columns={[
                  { key: 'cashier', header: t('reports.discounts.cashier'), render: (c) => cashier(c.cashier_name) },
                  { key: 'invoices', header: t('reports.discounts.invoices'), align: 'right', render: (c) => c.invoices },
                  { key: 'gross', header: t('reports.discounts.gross'), align: 'right', render: (c) => money(c.gross, lang) },
                  { key: 'automatic', header: t('reports.promotions.automatic'), align: 'right', render: (c) => money(c.automatic, lang) },
                  { key: 'coupon', header: t('reports.promotions.coupon'), align: 'right', render: (c) => money(c.coupon, lang) },
                  {
                    key: 'manual',
                    header: t('reports.promotions.manual'),
                    align: 'right',
                    card: 'amount',
                    render: (c) => <span className="font-semibold">{money(c.manual, lang)}</span>,
                  },
                ]}
              />
            </Section>
          </>
        )}
      </Loaded>
    </ReportFrame>
  )
}
