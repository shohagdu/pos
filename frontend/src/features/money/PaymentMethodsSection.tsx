import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'

import { type PaymentAccount, type PaymentMethod, type PaymentMethodPatch, moneyApi, moneyKeys, usePaymentMethods } from '../../api/money'
import { Alert } from '../../components/ui/Alert'
import { problemMessage } from '../../lib/problem'
import { QueryStatus } from './shared'

const SELECT =
  'min-h-10 w-full min-w-40 rounded-lg border border-border bg-surface px-2 text-sm outline-none focus:border-brand-500 focus:ring-2 focus:ring-brand-100'

/** Payment methods: default account, reference rule and on/off (accounts.manage). */
export function PaymentMethodsSection({ tenantId, accounts }: { tenantId: string; accounts: PaymentAccount[] }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const methods = usePaymentMethods(tenantId, true)
  const update = useMutation({
    mutationFn: ({ id, patch }: { id: string; patch: PaymentMethodPatch }) => moneyApi.updateMethod(id, patch),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: moneyKeys.all(tenantId) }),
  })
  const active = accounts.filter((a) => a.status === 'active')
  const change = (m: PaymentMethod, patch: PaymentMethodPatch) => update.mutate({ id: m.id, patch })

  return (
    <section className="flex flex-col gap-3" aria-labelledby="methods-heading">
      <div>
        <h2 id="methods-heading" className="text-lg font-semibold">
          {t('money.methods')}
        </h2>
        <p className="text-sm text-ink-muted">{t('money.methodsHint')}</p>
      </div>
      {update.isError && <Alert tone="danger">{problemMessage(update.error, t)}</Alert>}
      {!methods.data ? (
        <QueryStatus query={methods} />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-border bg-surface">
          <table className="w-full text-sm">
            <caption className="sr-only">{t('money.methods')}</caption>
            <thead className="border-b border-border bg-canvas text-xs text-ink-muted uppercase">
              <tr>
                <th scope="col" className="px-3 py-2.5 text-left">{t('money.method')}</th>
                <th scope="col" className="px-3 py-2.5 text-left">{t('money.defaultAccount')}</th>
                <th scope="col" className="px-3 py-2.5 text-center">{t('money.requiresReference')}</th>
                <th scope="col" className="px-3 py-2.5 text-center">{t('money.active')}</th>
              </tr>
            </thead>
            <tbody>
              {methods.data.map((m) => (
                <tr key={m.id} className="border-b border-border last:border-b-0">
                  <td className="px-3 py-2">
                    <div className="font-medium">{m.name}</div>
                    <div className="text-xs text-ink-muted">{t(`money.methodTypes.${m.method_type}`, { defaultValue: m.method_type })}</div>
                  </td>
                  <td className="px-3 py-2">
                    {m.method_type === 'customer_credit' ? (
                      <span className="text-ink-muted">—</span>
                    ) : (
                      <select
                        aria-label={t('money.defaultAccountFor', { name: m.name })}
                        className={SELECT}
                        value={m.default_account_id ?? ''}
                        disabled={update.isPending}
                        onChange={(e) => e.target.value && change(m, { default_account_id: e.target.value })}
                      >
                        {!m.default_account_id && <option value="">{t('money.noDefaultAccount')}</option>}
                        {active.map((a) => (
                          <option key={a.id} value={a.id}>
                            {a.name}
                          </option>
                        ))}
                        {m.default_account_id && !active.some((a) => a.id === m.default_account_id) && (
                          <option value={m.default_account_id}>{m.default_account_name ?? m.default_account_id}</option>
                        )}
                      </select>
                    )}
                  </td>
                  <td className="px-3 py-2 text-center">
                    <input
                      type="checkbox"
                      className="size-5 accent-brand-600"
                      aria-label={t('money.requiresReferenceFor', { name: m.name })}
                      checked={m.requires_reference}
                      disabled={update.isPending}
                      onChange={(e) => change(m, { requires_reference: e.target.checked })}
                    />
                  </td>
                  <td className="px-3 py-2 text-center">
                    <input
                      type="checkbox"
                      className="size-5 accent-brand-600"
                      aria-label={t('money.activeFor', { name: m.name })}
                      checked={m.is_active}
                      disabled={update.isPending}
                      onChange={(e) => change(m, { is_active: e.target.checked })}
                    />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}
