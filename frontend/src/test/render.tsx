import { render } from '@testing-library/react'
import { createMemoryRouter, RouterProvider } from 'react-router'

import { AppProviders } from '../app/providers'
import { createQueryClient } from '../app/queryClient'
import { routes } from '../app/routes'

/** Renders the real route tree at `path` with fresh providers. */
export function renderRoute(path: string) {
  const router = createMemoryRouter(routes, { initialEntries: [path] })
  const queryClient = createQueryClient()
  queryClient.setDefaultOptions({ queries: { retry: false } })
  return { router, ...render(<AppProviders queryClient={queryClient}><RouterProvider router={router} /></AppProviders>) }
}
