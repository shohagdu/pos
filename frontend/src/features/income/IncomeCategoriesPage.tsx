import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { type IncomeCategory, moneyApi, moneyKeys, useIncomeCategories } from '../../api/money'
import { Alert } from '../../components/ui/Alert'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Checkbox } from '../../components/ui/Checkbox'
import { type Column, DataTable } from '../../components/ui/DataTable'
import { Dialog } from '../../components/ui/Dialog'
import { PageHeader } from '../../components/ui/PageHeader'
import { TextField } from '../../components/ui/TextField'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { QueryStatus } from '../money/shared'
import { useTenant } from '../money/utils'

/** Income categories (`/incomes/categories`). */
export function IncomeCategoriesPage() {
  const { t } = useTranslation()
  const { tenantId, can } = useTenant()
  const canManage = can('accounts.manage')
  const categories = useIncomeCategories(tenantId)
  const [editing, setEditing] = useState<IncomeCategory | 'new' | null>(null)

  const columns: Column<IncomeCategory>[] = [
    {
      key: 'name',
      header: t('income.categoryName'),
      render: (c) => (
        <span className="inline-flex flex-wrap items-center gap-2">
          <span className="font-medium">{c.name}</span>
          {c.is_system && <Badge tone="info">{t('income.system')}</Badge>}
        </span>
      ),
    },
    { key: 'count', header: t('income.incomeCount'), align: 'right', render: (c) => c.income_count },
    {
      key: 'status',
      header: t('common.status'),
      render: (c) => <Badge tone={c.status === 'active' ? 'good' : 'neutral'}>{t(`income.categoryStatuses.${c.status}`)}</Badge>,
    },
    ...(canManage
      ? ([
          {
            key: 'actions',
            header: <span className="sr-only">{t('common.actions')}</span>,
            render: (c) => (
              <Button variant="ghost" className="min-h-9" onClick={() => setEditing(c)} aria-label={t('income.editCategoryName', { name: c.name })}>
                {t('common.edit')}
              </Button>
            ),
          },
        ] satisfies Column<IncomeCategory>[])
      : []),
  ]

  return (
    <div className="flex max-w-4xl flex-col gap-6">
      <Link to="/incomes" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('income.title')}
      </Link>
      <PageHeader
        title={t('income.categories')}
        subtitle={t('income.categoriesSubtitle')}
        actions={canManage && <Button onClick={() => setEditing('new')}>{t('income.addCategory')}</Button>}
      />
      {categories.isSuccess ? (
        <DataTable caption={t('income.categories')} columns={columns} rows={categories.data} rowKey={(c) => c.id} empty={t('income.noCategories')} />
      ) : (
        <QueryStatus query={categories} />
      )}
      {editing && (
        <CategoryDialog key={editing === 'new' ? 'new' : editing.id} category={editing === 'new' ? undefined : editing} tenantId={tenantId} onClose={() => setEditing(null)} />
      )}
    </div>
  )
}

function CategoryDialog({ category, tenantId, onClose }: { category?: IncomeCategory; tenantId: string; onClose: () => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [name, setName] = useState(category?.name ?? '')
  const [active, setActive] = useState(category ? category.status === 'active' : true)
  const save = useMutation({
    mutationFn: () =>
      category ? moneyApi.updateIncomeCategory(category.id, name.trim(), active ? 'active' : 'inactive') : moneyApi.createIncomeCategory(name.trim()),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: moneyKeys.all(tenantId) })
      onClose()
    },
  })
  const errors = fieldErrors(save.error, t)
  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate()
  }
  return (
    <Dialog open onClose={onClose} title={category ? t('income.editCategory') : t('income.addCategory')}>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-4">
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        <TextField label={t('income.categoryName')} value={name} onChange={(e) => setName(e.target.value)} required maxLength={100} autoFocus error={errors.name} />
        {category && (
          <Checkbox label={t('income.categoryActive')} hint={t('income.categoryActiveHint')} checked={active} onChange={(e) => setActive(e.target.checked)} />
        )}
        <div className="flex flex-wrap justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" disabled={save.isPending || !name.trim()}>
            {save.isPending ? t('common.saving') : t('common.save')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
