import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'

import { type ManualJournalInput, accountingApi, accountingKeys, useLedgerAccounts } from '../../api/accounting'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { TextField } from '../../components/ui/TextField'
import { today } from '../../lib/dates'
import { type Dec, ZERO, sum, toFixed, tryParse } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import { problemMessage } from '../../lib/problem'
import { accountLabel, chartOrder } from './ledger'

interface Line {
  id: number
  account_id: string
  debit: string
  credit: string
  memo: string
}

let nextId = 1
const blankLine = (): Line => ({ id: nextId++, account_id: '', debit: '', credit: '', memo: '' })

/** Parses an amount field: blank = 0, invalid or negative = null. */
function amount(text: string): Dec | null {
  if (text.trim() === '') return ZERO
  const v = tryParse(text)
  return v === null || v < 0n ? null : v
}

const isBlank = (l: Line) => !l.account_id && !l.debit.trim() && !l.credit.trim() && !l.memo.trim()

/** Manual journal entry (ACC-R02): balanced lines on postable accounts. */
export function NewJournalDialog({ tenantId, open, onClose }: { tenantId: string; open: boolean; onClose: () => void }) {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const accounts = useLedgerAccounts(tenantId)
  const postable = useMemo(() => chartOrder(accounts.data ?? []).filter((a) => a.is_postable && a.status === 'active'), [accounts.data])
  const [date, setDate] = useState(today())
  const [memo, setMemo] = useState('')
  const [lines, setLines] = useState<Line[]>(() => [blankLine(), blankLine()])

  const save = useMutation({
    mutationFn: (input: ManualJournalInput) => accountingApi.createJournal(input),
    onSuccess: (journal) => {
      void queryClient.invalidateQueries({ queryKey: accountingKeys.all(tenantId) })
      void queryClient.invalidateQueries({ queryKey: ['reports', tenantId] })
      onClose()
      void navigate(`/accounting/journals/${journal.id}`)
    },
  })

  const setLine = (id: number, patch: Partial<Line>) => setLines((ls) => ls.map((l) => (l.id === id ? { ...l, ...patch } : l)))
  const used = lines.filter((l) => !isBlank(l))
  const debits = used.map((l) => amount(l.debit))
  const credits = used.map((l) => amount(l.credit))
  const invalidAmount = debits.some((d) => d === null) || credits.some((c) => c === null)
  const totalDebit = sum(debits.map((d) => d ?? ZERO))
  const totalCredit = sum(credits.map((c) => c ?? ZERO))
  const difference = totalDebit - totalCredit
  const linesOk = used.every((l, i) => {
    const d = debits[i] ?? ZERO
    const c = credits[i] ?? ZERO
    return l.account_id !== '' && (d > 0n) !== (c > 0n)
  })
  const balanced = !invalidAmount && difference === 0n && totalDebit > 0n
  const valid = memo.trim() !== '' && used.length >= 2 && linesOk && balanced

  const money = (v: Dec) => formatMoney(toFixed(v, 2), i18n.language)

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    if (!valid || save.isPending) return
    save.mutate({
      date: date || undefined,
      memo: memo.trim(),
      lines: used.map((l) => ({
        account_id: l.account_id,
        ...((amount(l.debit) ?? ZERO) > 0n ? { debit: l.debit.trim() } : {}),
        ...((amount(l.credit) ?? ZERO) > 0n ? { credit: l.credit.trim() } : {}),
        ...(l.memo.trim() ? { memo: l.memo.trim() } : {}),
      })),
    })
  }

  const cell = 'min-h-10 w-full rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100'
  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={t('accounting.journal.new')}
      wide
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" form="new-journal" disabled={!valid || save.isPending}>
            {save.isPending ? t('common.saving') : t('accounting.journal.post')}
          </Button>
        </>
      }
    >
      <form id="new-journal" onSubmit={onSubmit} className="flex flex-col gap-4">
        <div className="grid gap-3 sm:grid-cols-[12rem_1fr]">
          <TextField label={t('common.date')} type="date" value={date} onChange={(e) => setDate(e.target.value)} required />
          <TextField label={t('accounting.journal.memo')} value={memo} onChange={(e) => setMemo(e.target.value)} required maxLength={500} />
        </div>
        <div className="overflow-x-auto">
          <table className="w-full min-w-[40rem] text-sm">
            <caption className="sr-only">{t('accounting.journal.lines')}</caption>
            <thead className="text-xs text-ink-muted uppercase">
              <tr>
                <th scope="col" className="px-1 py-1 text-left">
                  {t('accounting.journal.account')}
                </th>
                <th scope="col" className="w-32 px-1 py-1 text-right">
                  {t('accounting.debit')}
                </th>
                <th scope="col" className="w-32 px-1 py-1 text-right">
                  {t('accounting.credit')}
                </th>
                <th scope="col" className="px-1 py-1 text-left">
                  {t('accounting.journal.lineMemo')}
                </th>
                <th scope="col" className="w-10 px-1 py-1">
                  <span className="sr-only">{t('common.actions')}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {lines.map((l, i) => {
                const n = i + 1
                return (
                  <tr key={l.id}>
                    <td className="px-1 py-1">
                      <select
                        aria-label={t('accounting.journal.accountN', { n })}
                        className={cell}
                        value={l.account_id}
                        onChange={(e) => setLine(l.id, { account_id: e.target.value })}
                      >
                        <option value="">{t('accounting.journal.chooseAccount')}</option>
                        {postable.map((a) => (
                          <option key={a.id} value={a.id}>
                            {accountLabel(a)}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className="px-1 py-1">
                      <input
                        aria-label={t('accounting.journal.debitN', { n })}
                        inputMode="decimal"
                        className={`${cell} text-right tabular-nums`}
                        value={l.debit}
                        aria-invalid={amount(l.debit) === null ? true : undefined}
                        onChange={(e) => setLine(l.id, { debit: e.target.value, ...(e.target.value.trim() ? { credit: '' } : {}) })}
                      />
                    </td>
                    <td className="px-1 py-1">
                      <input
                        aria-label={t('accounting.journal.creditN', { n })}
                        inputMode="decimal"
                        className={`${cell} text-right tabular-nums`}
                        value={l.credit}
                        aria-invalid={amount(l.credit) === null ? true : undefined}
                        onChange={(e) => setLine(l.id, { credit: e.target.value, ...(e.target.value.trim() ? { debit: '' } : {}) })}
                      />
                    </td>
                    <td className="px-1 py-1">
                      <input aria-label={t('accounting.journal.memoN', { n })} className={cell} value={l.memo} onChange={(e) => setLine(l.id, { memo: e.target.value })} />
                    </td>
                    <td className="px-1 py-1 text-center">
                      <button
                        type="button"
                        className="rounded p-2 text-ink-muted hover:bg-canvas hover:text-danger disabled:opacity-40"
                        aria-label={t('accounting.journal.removeN', { n })}
                        disabled={lines.length <= 2}
                        onClick={() => setLines((ls) => ls.filter((x) => x.id !== l.id))}
                      >
                        ✕
                      </button>
                    </td>
                  </tr>
                )
              })}
            </tbody>
            <tfoot className="border-t-2 border-border font-semibold">
              <tr>
                <th scope="row" className="px-1 py-2 text-left">
                  {t('common.totals')}
                </th>
                <td className="px-2 py-2 text-right tabular-nums" data-testid="total-debit">
                  {money(totalDebit)}
                </td>
                <td className="px-2 py-2 text-right tabular-nums" data-testid="total-credit">
                  {money(totalCredit)}
                </td>
                <td colSpan={2} className="px-2 py-2" aria-live="polite">
                  {balanced ? (
                    <span className="text-brand-700">✓ {t('accounting.journal.balanced')}</span>
                  ) : (
                    <span className="text-danger">{t('accounting.journal.difference', { amount: money(difference < 0n ? -difference : difference) })}</span>
                  )}
                </td>
              </tr>
            </tfoot>
          </table>
        </div>
        <div>
          <Button variant="secondary" onClick={() => setLines((ls) => [...ls, blankLine()])}>
            {t('accounting.journal.addLine')}
          </Button>
        </div>
        <p className="text-xs text-ink-muted">{t('accounting.journal.help')}</p>
        {save.isError && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
      </form>
    </Dialog>
  )
}
