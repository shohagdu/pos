import { useInfiniteQuery } from '@tanstack/react-query'

import { apiFetch } from './client'
import { PAGE_SIZE, type Page, queryString } from './query'

export interface AuditRecord {
  id: string
  created_at: string
  actor_type: 'user' | 'platform_staff' | 'system' | 'api_key'
  actor_id: string | null
  actor_name: string | null
  action: string
  entity_type: string | null
  entity_id: string | null
  branch_id: string | null
  before: unknown
  after: unknown
  reason: string | null
  ip_address: string | null
  user_agent: string | null
  request_id: string | null
}

export interface AuditFilter {
  actor_id?: string
  /** Exact action or a prefix such as `sale.*`. */
  action?: string
  entity_type?: string
  entity_id?: string
  /** RFC 3339 timestamps: `from` inclusive, `to` exclusive. */
  from?: string
  to?: string
}

export const auditApi = {
  list: (filter: AuditFilter, cursor: string | null) =>
    apiFetch<Page<AuditRecord>>(`/api/v1/audit-logs${queryString({ ...filter, limit: PAGE_SIZE, cursor })}`),
}

export const useAuditLogs = (tenantId: string, filter: AuditFilter) =>
  useInfiniteQuery({
    queryKey: ['audit', tenantId, filter],
    queryFn: ({ pageParam }) => auditApi.list(filter, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next_cursor,
  })
