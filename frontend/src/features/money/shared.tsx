// Small helpers shared by the inventory, customers, money and expenses pages.

import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { Alert } from '../../components/ui/Alert'
import { Button } from '../../components/ui/Button'
import { Spinner } from '../../components/ui/Spinner'
import { problemMessage } from '../../lib/problem'

const LINK_VARIANTS = {
  primary: 'bg-brand-600 text-white hover:bg-brand-700',
  secondary: 'bg-surface text-ink border border-border hover:bg-canvas',
}

/** A router link that looks like a button. */
export function LinkButton({
  to,
  children,
  variant = 'secondary',
  className = '',
}: {
  to: string
  children: ReactNode
  variant?: keyof typeof LINK_VARIANTS
  className?: string
}) {
  return (
    <Link
      to={to}
      className={`inline-flex min-h-11 items-center justify-center gap-2 rounded-lg px-4 text-sm font-semibold transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-600 ${LINK_VARIANTS[variant]} ${className}`}
    >
      {children}
    </Link>
  )
}

/** Text link in the brand colour. */
export function TextLink({ to, children }: { to: string; children: ReactNode }) {
  return (
    <Link to={to} className="font-medium text-brand-700 hover:underline">
      {children}
    </Link>
  )
}

interface Paged {
  hasNextPage: boolean
  isFetchingNextPage: boolean
  fetchNextPage: () => unknown
}

/** "Load more" under a cursor-paged list. */
export function LoadMore({ query }: { query: Paged }) {
  const { t } = useTranslation()
  if (!query.hasNextPage) return null
  return (
    <div className="flex justify-center print:hidden">
      <Button variant="secondary" disabled={query.isFetchingNextPage} onClick={() => void query.fetchNextPage()}>
        {query.isFetchingNextPage ? t('common.loading') : t('common.loadMore')}
      </Button>
    </div>
  )
}

/** Spinner or error alert while a query is not ready; `null` once it has data. */
export function QueryStatus({ query }: { query: { isPending: boolean; isError: boolean; error: unknown } }) {
  const { t } = useTranslation()
  if (query.isPending) return <Spinner />
  if (query.isError) return <Alert tone="danger">{problemMessage(query.error, t)}</Alert>
  return null
}

