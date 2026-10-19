import { useQuery } from '@tanstack/react-query'

import { apiFetch } from './client'

export interface HealthStatus {
  status: string
}

export interface Meta {
  name: string
  version: string
  environment: string
}

export const systemKeys = {
  readiness: ['system', 'readiness'] as const,
  meta: ['system', 'meta'] as const,
}

export function useReadiness() {
  return useQuery({
    queryKey: systemKeys.readiness,
    queryFn: () => apiFetch<HealthStatus>('/readyz'),
    refetchInterval: 30_000,
    retry: false,
  })
}

export function useMeta() {
  return useQuery({ queryKey: systemKeys.meta, queryFn: () => apiFetch<Meta>('/api/v1/meta'), retry: 1 })
}
