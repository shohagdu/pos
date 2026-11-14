import type { Member } from '../../api/users'
import { useSession } from '../auth/session'

/** Whether the signed-in owner may sign in as `member` (mirrors the API rules). */
export function useCanImpersonate(member: Member): boolean {
  const { me } = useSession()
  return me?.tenant?.is_owner === true && !me.impersonator && !member.is_owner && member.status === 'active' && member.id !== me.tenant.membership_id
}
