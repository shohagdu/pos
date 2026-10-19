import { useMutation } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type BillingCycle, type PlanOffer, type SubscriptionInvoice, billingApi, useBillingOverview } from '../../api/billing'
import { Alert } from '../../components/ui/Alert'
import { Card } from '../../components/ui/Card'
import { Dialog } from '../../components/ui/Dialog'
import { Spinner } from '../../components/ui/Spinner'
import { problemMessage } from '../../lib/problem'
import { ClaimForm } from './ClaimForm'
import { DataExport } from './DataExport'
import { Invoices } from './Invoices'
import { Renewal } from './Renewal'

/** The owner's part of the Plan page: renew/upgrade, invoices and payments, data export. */
export function Billing({ tenantId, currentPlan }: { tenantId: string; currentPlan: string | null }) {
  const { t } = useTranslation()
  const overview = useBillingOverview(tenantId)
  const [cycle, setCycle] = useState<BillingCycle>('monthly')
  const [invoice, setInvoice] = useState<SubscriptionInvoice | null>(null)
  const [submitted, setSubmitted] = useState(false)

  const request = useMutation({
    mutationFn: (plan: PlanOffer) => billingApi.requestInvoice(plan.code, cycle),
    onSuccess: (inv) => {
      setSubmitted(false)
      setInvoice(inv)
    },
  })

  return (
    <div className="flex flex-col gap-6">
      {submitted && <Alert tone="success">{t('billing.claim.submitted')}</Alert>}
      {overview.isPending ? (
        <Spinner />
      ) : overview.isError ? (
        <Alert tone="danger">{problemMessage(overview.error, t)}</Alert>
      ) : (
        <>
          <Card>
            <Renewal
              plans={overview.data.plans}
              currentPlan={currentPlan}
              cycle={cycle}
              onCycle={setCycle}
              busyPlan={request.isPending ? (request.variables?.code ?? null) : null}
              onChoose={(plan) => request.mutate(plan)}
            />
            {request.isError && (
              <Alert tone="danger" className="mt-4">
                {problemMessage(request.error, t)}
              </Alert>
            )}
          </Card>
          <Card>
            <Invoices
              invoices={overview.data.invoices}
              claims={overview.data.claims}
              onReport={(inv) => {
                setSubmitted(false)
                setInvoice(inv)
              }}
            />
          </Card>
          <Dialog open={invoice !== null} onClose={() => setInvoice(null)} title={t('billing.claim.title')} wide>
            {invoice && (
              <ClaimForm
                key={invoice.id}
                tenantId={tenantId}
                invoice={invoice}
                instructions={overview.data.payment_instructions ?? {}}
                onCancel={() => setInvoice(null)}
                onSubmitted={() => {
                  setInvoice(null)
                  setSubmitted(true)
                }}
              />
            )}
          </Dialog>
        </>
      )}
      <DataExport />
    </div>
  )
}
