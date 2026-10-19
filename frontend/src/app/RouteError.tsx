import { useTranslation } from 'react-i18next'
import { isRouteErrorResponse, Link, useRouteError } from 'react-router'

export function RouteError() {
  const { t } = useTranslation()
  const error = useRouteError()
  const notFound = isRouteErrorResponse(error) && error.status === 404
  return (
    <div className="mx-auto flex min-h-svh max-w-md flex-col items-center justify-center gap-4 px-4 text-center">
      <h1 className="text-2xl font-bold">{notFound ? t('errors.notFound') : t('errors.unexpected')}</h1>
      <Link to="/" className="font-semibold text-brand-700 hover:underline">
        {t('common.backHome')}
      </Link>
    </div>
  )
}
