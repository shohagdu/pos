import { QueryClient } from '@tanstack/react-query'

import { ApiError } from '../api/client'

export function createQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        // Never retry client errors (4xx); they will not succeed on retry.
        retry: (count, error) => !(error instanceof ApiError && error.status < 500) && count < 2,
      },
    },
  })
}
