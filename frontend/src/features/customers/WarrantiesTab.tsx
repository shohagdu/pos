import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type WarrantyFilter, useWarranties } from '../../api/sales'
import { LoadMore, QueryStatus } from '../money/shared'
import { useTenant } from '../money/utils'
import { WarrantyStatusFilter } from '../sales/WarrantyLookupPage'
import { WarrantyTable } from '../sales/WarrantyTable'

/** Every item the customer bought with a warranty, and whether it still covers a claim. */
export function WarrantiesTab({ customerId }: { customerId: string }) {
  const { t } = useTranslation()
  const { tenantId } = useTenant()
  const [status, setStatus] = useState<NonNullable<WarrantyFilter['status']>>('')
  const warranties = useWarranties(tenantId, { customer_id: customerId, status })
  const pages = warranties.data?.pages ?? []

  return (
    <div className="flex flex-col gap-4">
      <WarrantyStatusFilter value={status} onChange={setStatus} />
      {warranties.isSuccess ? (
        <>
          <WarrantyTable
            rows={pages.flatMap((p) => p.data)}
            today={pages[0]?.today ?? ''}
            caption={t('customers.warranties')}
            empty={t('customers.noWarranties')}
          />
          <LoadMore query={warranties} />
        </>
      ) : (
        <QueryStatus query={warranties} />
      )}
    </div>
  )
}
