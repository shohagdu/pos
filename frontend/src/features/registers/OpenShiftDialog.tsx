import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { type Register, registerKeys, registersApi } from '../../api/registers'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Dialog } from '../../components/ui/Dialog'
import { Select } from '../../components/ui/Select'
import { TextField } from '../../components/ui/TextField'
import { tryParse } from '../../lib/decimal'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'

/** Opens a shift on a free register with the cash in the drawer. Mount only while open. */
export function OpenShiftDialog({ tenantId, registers, onClose }: { tenantId: string; registers: Register[]; onClose: () => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const free = registers.filter((r) => r.status === 'active' && !r.open_shift_id)
  const [registerId, setRegisterId] = useState(free[0]?.id ?? '')
  const [cash, setCash] = useState('0')
  const [localErrors, setLocalErrors] = useState<Record<string, string>>({})

  const open = useMutation({
    mutationFn: () => registersApi.openShift(registerId, cash.trim()),
    onSuccess: async (shift) => {
      queryClient.setQueryData(registerKeys.current(tenantId), shift)
      await queryClient.invalidateQueries({ queryKey: registerKeys.all(tenantId) })
      onClose()
    },
  })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    const errs: Record<string, string> = {}
    if (!registerId) errs.register_id = t('shifts.errors.register')
    const value = tryParse(cash)
    if (value === null || value < 0n) errs.opening_cash = t('shifts.errors.cash')
    setLocalErrors(errs)
    if (Object.keys(errs).length === 0) open.mutate()
  }

  const errors = { ...fieldErrors(open.error, t), ...localErrors }
  return (
    <Dialog open onClose={onClose} title={t('shifts.open')}>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {open.isError && hasFormError(open.error) && <Alert tone="danger">{problemMessage(open.error, t)}</Alert>}
        {free.length === 0 ? (
          <Alert tone="warning">{t('shifts.noFreeRegister')}</Alert>
        ) : (
          <Select
            label={t('shifts.register')}
            value={registerId}
            onChange={(e) => setRegisterId(e.target.value)}
            error={errors.register_id}
            options={free.map((r) => ({ value: r.id, label: r.name }))}
          />
        )}
        <TextField
          label={t('shifts.openingCash')}
          value={cash}
          onChange={(e) => setCash(e.target.value)}
          inputMode="decimal"
          hint={t('shifts.openingCashHint')}
          error={errors.opening_cash}
        />
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={open.isPending || free.length === 0}>
            {open.isPending ? t('common.saving') : t('shifts.open')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
