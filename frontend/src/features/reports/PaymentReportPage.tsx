import { useTranslation } from 'react-i18next'

import { useReport } from '../../api/reports'
import { DataTable } from '../../components/ui/DataTable'
import { DateRangeFilter } from '../../components/ui/DateRangeFilter'
import { downloadCsv } from '../../lib/csv'
import { useSession } from '../auth/session'
import { reportAccess } from './access'
import { minus, money, total, usePeriodLabel, useRange } from './format'
import { type ExportAction, Loaded, ReportFrame, Section } from './shared'

/** Payment report (REP-08): money in/out by method and by account. */
export function PaymentReportPage() {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const { me, can } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const allowed = reportAccess(can).payments
  const [range, setRange] = useRange()
  const periodLabel = usePeriodLabel()
  const report = useReport(tenantId, 'payments', range, allowed)

  const exports: ExportAction[] = []
  const d = report.data
  if (d) {
    const file = (base: string) => `${base}-${range.from}-${range.to}`
    exports.push(
      {
        label: t('reports.payments.byMethod'),
        run: () =>
          downloadCsv(file('payments-by-method'), d.by_method, [
            { header: t('reports.method'), value: (r) => r.method_name },
            { header: t('reports.moneyIn'), value: (r) => r.money_in },
            { header: t('reports.moneyOut'), value: (r) => r.money_out },
            { header: t('reports.net'), value: (r) => minus(r.money_in, r.money_out) },
            { header: t('reports.count'), value: (r) => r.count },
          ]),
      },
      {
        label: t('reports.payments.byAccount'),
        run: () =>
          downloadCsv(file('payments-by-account'), d.by_account, [
            { header: t('reports.payments.account'), value: (r) => r.account_name },
            { header: t('reports.moneyIn'), value: (r) => r.money_in },
            { header: t('reports.moneyOut'), value: (r) => r.money_out },
            { header: t('reports.net'), value: (r) => minus(r.money_in, r.money_out) },
          ]),
      },
    )
  }

  return (
    <ReportFrame
      title={t('reports.payments.title')}
      allowed={allowed}
      period={periodLabel(range.from, range.to)}
      exports={exports}
      updatedAt={report.dataUpdatedAt}
      filters={<DateRangeFilter from={range.from} to={range.to} onChange={setRange} />}
    >
      <Loaded query={report}>
        {(r) => {
          const inM = total(r.by_method.map((m) => m.money_in)) ?? '0'
          const outM = total(r.by_method.map((m) => m.money_out)) ?? '0'
          const inA = total(r.by_account.map((m) => m.money_in)) ?? '0'
          const outA = total(r.by_account.map((m) => m.money_out)) ?? '0'
          return (
            <>
              <Section title={t('reports.payments.byMethod')}>
                <DataTable
                  caption={t('reports.payments.byMethod')}
                  rows={r.by_method}
                  rowKey={(m) => m.method_name}
                  empty={t('reports.noData')}
                  columns={[
                    { key: 'method', header: t('reports.method'), render: (m) => m.method_name },
                    { key: 'in', header: t('reports.moneyIn'), align: 'right', render: (m) => money(m.money_in, lang) },
                    { key: 'out', header: t('reports.moneyOut'), align: 'right', render: (m) => money(m.money_out, lang) },
                    { key: 'net', header: t('reports.net'), align: 'right', render: (m) => money(minus(m.money_in, m.money_out), lang) },
                    { key: 'count', header: t('reports.count'), align: 'right', render: (m) => m.count },
                  ]}
                  footer={[
                    t('common.totals'),
                    money(inM, lang),
                    money(outM, lang),
                    money(minus(inM, outM), lang),
                    r.by_method.reduce((s, m) => s + m.count, 0),
                  ]}
                />
              </Section>
              <Section title={t('reports.payments.byAccount')}>
                <DataTable
                  caption={t('reports.payments.byAccount')}
                  rows={r.by_account}
                  rowKey={(a) => a.account_id}
                  empty={t('reports.noData')}
                  columns={[
                    { key: 'account', header: t('reports.payments.account'), render: (a) => a.account_name },
                    { key: 'in', header: t('reports.moneyIn'), align: 'right', render: (a) => money(a.money_in, lang) },
                    { key: 'out', header: t('reports.moneyOut'), align: 'right', render: (a) => money(a.money_out, lang) },
                    { key: 'net', header: t('reports.net'), align: 'right', render: (a) => money(minus(a.money_in, a.money_out), lang) },
                  ]}
                  footer={[t('common.totals'), money(inA, lang), money(outA, lang), money(minus(inA, outA), lang)]}
                />
              </Section>
            </>
          )
        }}
      </Loaded>
    </ReportFrame>
  )
}
