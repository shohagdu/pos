import { useTranslation } from 'react-i18next'

import { Button } from '../../components/ui/Button'
import { ZERO, dec } from '../../lib/decimal'
import { formatMoney } from '../../lib/format'
import type { CartCustomer } from './cart'

/** The cart's customer with a "change" button (F4). */
export function CustomerPanel({ customer, onChange }: { customer: CartCustomer | null; onChange: () => void }) {
  const { t, i18n } = useTranslation()
  return (
    <div className="flex items-center justify-between gap-3">
      <div className="min-w-0">
        <div className="text-xs font-medium text-ink-muted uppercase">{t('pos.customer')}</div>
        <div className="truncate font-semibold" data-testid="pos-customer">
          {customer ? customer.name : t('pos.walkIn')}
        </div>
        {customer?.phone && <div className="text-xs text-ink-muted">{customer.phone}</div>}
        {customer && dec(customer.balance) > ZERO && (
          <div className="text-xs font-medium text-danger">{t('pos.customerDue', { amount: formatMoney(customer.balance, i18n.language) })}</div>
        )}
      </div>
      <Button variant="secondary" className="min-h-9 shrink-0" onClick={onChange}>
        {t('pos.changeCustomer')} <kbd className="text-xs text-ink-muted">F4</kbd>
      </Button>
    </div>
  )
}
