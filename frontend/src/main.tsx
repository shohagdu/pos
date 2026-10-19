import './styles/index.css'
import './i18n'

import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { createBrowserRouter, RouterProvider } from 'react-router'

import { AppProviders } from './app/providers'
import { createQueryClient } from './app/queryClient'
import { routes } from './app/routes'

const router = createBrowserRouter(routes)
const queryClient = createQueryClient()

const root = document.getElementById('root')
if (!root) throw new Error('#root element missing from index.html')

createRoot(root).render(
  <StrictMode>
    <AppProviders queryClient={queryClient}>
      <RouterProvider router={router} />
    </AppProviders>
  </StrictMode>,
)
