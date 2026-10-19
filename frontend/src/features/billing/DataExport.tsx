import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { downloadExport } from '../../api/billing'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'

/** NFR-PRIV-01: the owner downloads every record of the business. */
export function DataExport() {
  const { t } = useTranslation()
  const [busy, setBusy] = useState(false)
  const [failed, setFailed] = useState(false)

  async function download() {
    setBusy(true)
    setFailed(false)
    try {
      await downloadExport()
    } catch {
      setFailed(true)
    } finally {
      setBusy(false)
    }
  }

  return (
    <Card className="flex flex-col gap-3">
      <h2 className="text-lg font-semibold">{t('billing.export.title')}</h2>
      <p className="text-sm text-ink-muted">{t('billing.export.body')}</p>
      {failed && <Alert tone="danger">{t('billing.export.failed')}</Alert>}
      <Button variant="secondary" className="sm:self-start" disabled={busy} onClick={() => void download()}>
        {busy ? t('billing.export.preparing') : t('billing.export.button')}
      </Button>
    </Card>
  )
}
