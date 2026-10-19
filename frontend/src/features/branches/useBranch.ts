import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useSyncExternalStore } from 'react'

import { getBranchId, setBranchId, subscribeBranch } from '../../api/session'
import { type Branch, useBranches } from '../../api/tenancy'
import { useSession } from '../auth/session'

/**
 * The branch the member works in (`X-Branch-Id`) and the branches they may
 * pick. `branchId: null` = no header: postings go to the default branch and
 * lists show every allowed branch.
 */
export function useBranch() {
  const queryClient = useQueryClient()
  const { me } = useSession()
  const tenantId = me?.tenant?.tenant_id ?? ''
  const stored = useSyncExternalStore(subscribeBranch, getBranchId)
  const branches = useBranches(tenantId)
  const scope = me?.tenant?.branch_ids ?? null
  const choices: Branch[] = (branches.data ?? []).filter((b) => b.status === 'active' && (scope === null || scope.includes(b.id)))
  // A remembered branch that was deactivated or taken out of scope no longer counts.
  const branchId = stored && (!branches.data || choices.some((b) => b.id === stored)) ? stored : null
  const branch = choices.find((b) => b.id === branchId) ?? null

  const select = useCallback(
    (id: string | null) => {
      setBranchId(id)
      // Every list and figure is per branch: refetch them all, but keep the session.
      void queryClient.resetQueries({ predicate: (q) => q.queryKey[0] !== 'auth' && q.queryKey[0] !== 'tenancy' })
    },
    [queryClient],
  )

  return {
    branchId,
    branch,
    choices,
    /** Members not limited to some branches may also work across all of them. */
    canSeeAll: scope === null,
    select,
  }
}
