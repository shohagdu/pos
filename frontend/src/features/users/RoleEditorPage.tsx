import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams, useSearchParams } from 'react-router'

import { ApiError } from '../../api/client'
import { type Permission, type Role, usePermissions, useRoles, usersApi, usersKeys } from '../../api/users'
import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmButton } from '../../components/ui/ConfirmButton'
import { Spinner } from '../../components/ui/Spinner'
import { TextField } from '../../components/ui/TextField'
import { fieldErrors, hasFormError, problemMessage } from '../../lib/problem'
import { useSession } from '../auth/session'
import { groupByModule, moduleLabel, permissionLabel, roleName } from './labels'

/** Granted to the owner only; never offered in custom roles. */
const OWNER_ONLY = new Set(['subscription.manage'])

/** `/users/roles/new[?from=<role id>]` and `/users/roles/:id`. */
export function RoleEditorPage() {
  const { t } = useTranslation()
  const { id } = useParams()
  const [search] = useSearchParams()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const roles = useRoles(tenantId)
  const catalogue = usePermissions()
  if (roles.isPending || catalogue.isPending) return <Spinner />
  if (roles.isError || catalogue.isError) return <Alert tone="danger">{problemMessage(roles.error ?? catalogue.error, t)}</Alert>

  const role = id ? roles.data.find((r) => r.id === id) : undefined
  if (id && !role) return <Alert tone="danger">{t('problems.NOT_FOUND')}</Alert>
  const template = roles.data.find((r) => r.id === search.get('from'))
  return <RoleForm key={role ? `edit-${role.id}` : `new-${template?.id ?? ''}`} role={role} template={template} catalogue={catalogue.data} tenantId={tenantId} />
}

function RoleForm({ role, template, catalogue, tenantId }: { role?: Role; template?: Role; catalogue: Permission[]; tenantId: string }) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { can, me } = useSession()
  const readOnly = role?.is_system === true || !can('users.manage_roles')
  const source = role ?? template
  const [name, setName] = useState(role ? roleName(t, role) : template ? t('roles.copyOf', { name: roleName(t, template) }) : '')
  const [description, setDescription] = useState(source?.description ?? '')
  const [discount, setDiscount] = useState(source?.max_discount_pct ?? '')
  const [selected, setSelected] = useState(() => new Set((source?.permissions ?? []).filter((p) => !OWNER_ONLY.has(p))))
  // Non-owners can only grant what they hold; show the rest as unavailable.
  const grantable = (code: string) => !OWNER_ONLY.has(code) && (me?.tenant?.is_owner === true || can(code))

  const refresh = () => queryClient.invalidateQueries({ queryKey: usersKeys.roles(tenantId) })
  const save = useMutation({
    mutationFn: () => {
      const input = { name, description, permissions: [...selected], max_discount_pct: discount.trim() === '' ? null : discount.trim() }
      return role ? usersApi.updateRole(role.id, role.version, input) : usersApi.createRole(input)
    },
    onSuccess: async (saved) => {
      await refresh()
      navigate(`/users/roles/${saved.id}`, { replace: true })
    },
    onError: (err) => {
      if (err instanceof ApiError && err.status === 412) void refresh()
    },
  })
  const remove = useMutation({
    mutationFn: () => usersApi.deleteRole(role?.id ?? ''),
    onSuccess: async () => {
      await refresh()
      navigate('/users/roles', { replace: true })
    },
  })
  const errors = fieldErrors(save.error, t)

  const setMany = (codes: string[], on: boolean) =>
    setSelected((prev) => {
      const next = new Set(prev)
      for (const c of codes) {
        if (on) next.add(c)
        else next.delete(c)
      }
      return next
    })

  function onSubmit(e: FormEvent) {
    e.preventDefault()
    save.mutate()
  }

  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <Link to="/users/roles" className="text-sm font-medium text-brand-700 hover:underline">
        ← {t('users.tabs.roles')}
      </Link>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h1 className="text-2xl font-bold">{role ? roleName(t, role) : t('roles.new')}</h1>
        {role?.is_system && can('users.manage_roles') && (
          <Link
            to={`/users/roles/new?from=${role.id}`}
            className="inline-flex min-h-11 items-center rounded-lg border border-border bg-surface px-4 text-sm font-semibold hover:bg-canvas"
          >
            {t('roles.duplicate')}
          </Link>
        )}
      </div>
      {role?.is_system && <Alert>{t('roles.systemReadOnly')}</Alert>}
      <form onSubmit={onSubmit} className="flex flex-col gap-6" noValidate>
        <Card className="flex flex-col gap-4">
          <TextField label={t('roles.name')} value={name} onChange={(e) => setName(e.target.value)} disabled={readOnly} required error={errors.name} />
          <TextField label={t('roles.description')} value={description} onChange={(e) => setDescription(e.target.value)} disabled={readOnly} error={errors.description} />
          <TextField
            label={t('roles.maxDiscount')}
            inputMode="decimal"
            value={discount}
            onChange={(e) => setDiscount(e.target.value)}
            disabled={readOnly}
            hint={t('roles.maxDiscountHint')}
            error={errors.max_discount_pct}
          />
        </Card>
        <Card className="flex flex-col gap-5">
          <div className="flex items-baseline justify-between gap-2">
            <h2 className="font-semibold">{t('roles.permissions')}</h2>
            <span className="text-sm text-ink-muted">{t('roles.selectedCount', { count: selected.size })}</span>
          </div>
          {errors.permissions && <p className="text-sm text-danger">{errors.permissions}</p>}
          {groupByModule(catalogue).map(([module, perms]) => {
            const codes = perms.filter((p) => grantable(p.code)).map((p) => p.code)
            const allOn = codes.length > 0 && codes.every((c) => selected.has(c))
            return (
              <fieldset key={module} className="flex flex-col gap-2 border-t border-border pt-4 first-of-type:border-t-0 first-of-type:pt-0">
                <legend className="flex w-full items-center justify-between gap-2">
                  <span className="text-sm font-semibold">{moduleLabel(t, module)}</span>
                  {!readOnly && codes.length > 0 && (
                    <button type="button" className="text-xs font-semibold text-brand-700 hover:underline" onClick={() => setMany(codes, !allOn)}>
                      {allOn ? t('roles.clearModule') : t('roles.selectModule')}
                    </button>
                  )}
                </legend>
                <div className="grid gap-2 sm:grid-cols-2">
                  {perms.map((p) => (
                    <label key={p.code} className={`flex items-start gap-2 text-sm ${grantable(p.code) ? '' : 'text-ink-muted'}`}>
                      <input
                        type="checkbox"
                        className="mt-0.5 size-4 accent-brand-600"
                        checked={selected.has(p.code)}
                        disabled={readOnly || !grantable(p.code)}
                        onChange={(e) => setMany([p.code], e.target.checked)}
                      />
                      <span>
                        {permissionLabel(t, p)}
                        {p.is_sensitive && <span className="ml-1 text-xs font-semibold text-warning">{t('roles.sensitive')}</span>}
                        <span className="block font-mono text-xs text-ink-muted">{p.code}</span>
                      </span>
                    </label>
                  ))}
                </div>
              </fieldset>
            )
          })}
        </Card>
        {save.isError && hasFormError(save.error) && <Alert tone="danger">{problemMessage(save.error, t)}</Alert>}
        {remove.isError && <Alert tone="danger">{problemMessage(remove.error, t)}</Alert>}
        {!readOnly && (
          <div className="flex flex-wrap items-center justify-between gap-3">
            {role ? (
              <ConfirmButton disabled={remove.isPending} onConfirm={() => remove.mutate()}>
                {t('roles.delete')}
              </ConfirmButton>
            ) : (
              <span />
            )}
            <Button type="submit" disabled={save.isPending}>
              {save.isPending ? t('common.saving') : t('common.save')}
            </Button>
          </div>
        )}
      </form>
    </div>
  )
}
