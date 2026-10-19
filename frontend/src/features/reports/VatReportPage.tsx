import { useTranslation } from 'react-i18next'

import { type VatRow, useReport } from '../../api/reports'
import { Alert } from '../../components/ui/Alert'
import { DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { StatCard } from '../../components/ui/StatCard'
import { downloadCsv } from '../../lib/csv'
import { dec } from '../../lib/decimal'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { money, pct, total, usePeriodLabel, useRange } from './format'
import { type ExportAction, Loaded, ReportFrame, Section } from './shared'

/** VAT summary (REP-11): output VAT on sales, input VAT on purchases and expenses, net. */
export function VatReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).vat
  const [range, setRange] = useRange()
  const periodLabel = usePeriodLabel()
  const report = useReport(tenantId, 'vatSummary', range, allowed)

  // `rate_pct: null` on the input side is VAT paid on expenses.
  const rateLabel = (r: VatRow, side: 'output' | 'input') =>
    r.rate_pct === null ? (side === 'input' ? t('reports.vat.expensesRow') : t('reports.vat.noRate')) : pct(r.rate_pct)

  const exports: ExportAction[] = []
  const d = report.data
  if (d) {
    const rows = [...d.output.map((r) => ({ side: t('reports.vat.outputVat'), label: rateLabel(r, 'output'), ...r })), ...d.input.map((r) => ({ side: t('reports.vat.inputVat'), label: rateLabel(r, 'input'), ...r }))]
    exports.push({
      run: () =>
        downloadCsv(`vat-summary-${range.from}-${range.to}`, rows, [
          { header: t('reports.vat.vat'), value: (r) => r.side },
          { header: t('reports.vat.rate'), value: (r) => r.label },
          { header: t('reports.vat.taxable'), value: (r) => r.taxable },
          { header: t('reports.vat.vat'), value: (r) => r.vat },
        ]),
    })
  }

  const table = (rows: VatRow[], side: 'output' | 'input', caption: string, vat: string) => (
    <DataTable
      caption={caption}
      rows={rows}
      rowKey={(r) => `${side}-${r.rate_pct ?? 'none'}`}
      empty={t('reports.noData')}
      columns={[
        { key: 'rate', header: t('reports.vat.rate'), render: (r) => rateLabel(r, side) },
        { key: 'taxable', header: t('reports.vat.taxable'), align: 'right', render: (r) => money(r.taxable, lang) },
        { key: 'vat', header: t('reports.vat.vat'), align: 'right', render: (r) => money(r.vat, lang) },
      ]}
      footer={[t('common.totals'), money(total(rows.map((r) => r.taxable)), lang), money(vat, lang)]}
    />
  )

  return (
    <ReportFrame
      title={t('reports.vat.title')}
      allowed={allowed}
      period={periodLabel(range.from, range.to)}
      exports={exports}
      updatedAt={report.dataUpdatedAt}
      filters={<DateRangeFilter from={range.from} to={range.to} onChange={setRange} />}
    >
      <Loaded query={report}>
        {(r) => {
          const refundable = dec(r.net_vat) < 0n
          return (
            <>
              {!r.is_vat_registered && <Alert tone="info">{t('reports.vat.notRegistered')}</Alert>}
              <div className="grid gap-3 sm:grid-cols-3">
                <StatCard label={t('reports.vat.outputVat')} value={money(r.output_vat, lang)} />
                <StatCard label={t('reports.vat.inputVat')} value={money(r.input_vat, lang)} />
                <StatCard
                  label={refundable ? t('reports.vat.netRefundable') : t('reports.vat.netPayable')}
                  value={money(refundable ? r.net_vat.replace('-', '') : r.net_vat, lang)}
                  tone={refundable ? 'good' : 'default'}
                />
              </div>
              <Section title={t('reports.vat.output')}>{table(r.output, 'output', t('reports.vat.output'), r.output_vat)}</Section>
              <Section title={t('reports.vat.input')}>{table(r.input, 'input', t('reports.vat.input'), r.input_vat)}</Section>
            </>
          )
        }}
      </Loaded>
    </ReportFrame>
  )
}
