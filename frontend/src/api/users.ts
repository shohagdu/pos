import { useQuery } from '@tanstack/react-query'

import type { SessionInfo } from './auth'
import { apiFetch } from './client'

export interface AssignedRole {
  role_id: string
  code: string
  name: string
  branch_id: string | null
}

export interface Member {
  id: string
  user_id: string
  full_name: string
  phone: string | null
  email: string | null
  status: 'invited' | 'active' | 'suspended' | 'removed'
  is_owner: boolean
  roles: AssignedRole[]
  joined_at: string | null
  last_login_at: string | null
}

export interface RoleAssignment {
  role_id: string
  branch_id: string | null
}

export interface Invitation {
  id: string
  destination: string
  roles: { id: string; name: string }[]
  branch_ids: string[]
  status: 'pending' | 'accepted' | 'revoked' | 'expired'
  expires_at: string
  created_at: string
  invited_by: string
}

export interface InvitationCreated {
  invitation: Invitation
  dev_link?: string
}

export interface InvitationPreview {
  business_name: string
  invited_by: string
  destination: string
  destination_kind: 'phone' | 'email'
  roles: string[]
  status: Invitation['status']
  expires_at: string
}

export interface Permission {
  code: string
  module: string
  description: string
  is_sensitive: boolean
}

export interface Role {
  id: string
  code: string
  name: string
  description: string | null
  is_system: boolean
  max_discount_pct: string | null
  permissions: string[]
  member_count: number
  version: number
}

export interface RoleInput {
  name: string
  description?: string
  permissions: string[]
  max_discount_pct?: string | null
}

export const usersApi = {
  members: () => apiFetch<Member[]>('/api/v1/members'),
  member: (id: string) => apiFetch<Member>(`/api/v1/members/${id}`),
  setRoles: (id: string, assignments: RoleAssignment[]) =>
    apiFetch<Member>(`/api/v1/members/${id}/roles`, { method: 'PUT', body: { assignments } }),
  suspend: (id: string) => apiFetch<Member>(`/api/v1/members/${id}/suspend`, { method: 'POST' }),
  reactivate: (id: string) => apiFetch<Member>(`/api/v1/members/${id}/reactivate`, { method: 'POST' }),
  remove: (id: string) => apiFetch<void>(`/api/v1/members/${id}`, { method: 'DELETE' }),
  memberSessions: (id: string) => apiFetch<SessionInfo[]>(`/api/v1/members/${id}/sessions`),
  revokeMemberSessions: (id: string) => apiFetch<{ revoked: number }>(`/api/v1/members/${id}/sessions`, { method: 'DELETE' }),

  invitations: () => apiFetch<Invitation[]>('/api/v1/invitations'),
  invite: (destination: string, roleIds: string[], branchIds: string[]) =>
    apiFetch<InvitationCreated>('/api/v1/invitations', { method: 'POST', body: { destination, role_ids: roleIds, branch_ids: branchIds } }),
  resendInvitation: (id: string) => apiFetch<InvitationCreated>(`/api/v1/invitations/${id}/resend`, { method: 'POST' }),
  revokeInvitation: (id: string) => apiFetch<void>(`/api/v1/invitations/${id}`, { method: 'DELETE' }),
  // The token travels in the body, never in a URL (request paths are logged).
  previewInvitation: (token: string) =>
    apiFetch<InvitationPreview>('/api/v1/invitations/preview', { method: 'POST', body: { token }, auth: false }),
  acceptInvitation: (token: string) =>
    apiFetch<{ tenant_id: string; membership_id: string }>('/api/v1/invitations/accept', { method: 'POST', body: { token } }),

  permissions: () => apiFetch<Permission[]>('/api/v1/permissions'),
  roles: () => apiFetch<Role[]>('/api/v1/roles'),
  createRole: (input: RoleInput) => apiFetch<Role>('/api/v1/roles', { method: 'POST', body: input }),
  updateRole: (id: string, version: number, input: RoleInput) =>
    apiFetch<Role>(`/api/v1/roles/${id}`, { method: 'PUT', body: input, ifMatch: version }),
  deleteRole: (id: string) => apiFetch<void>(`/api/v1/roles/${id}`, { method: 'DELETE' }),
}

export const usersKeys = {
  all: (tenantId: string) => ['users', tenantId] as const,
  members: (tenantId: string) => ['users', tenantId, 'members'] as const,
  member: (tenantId: string, id: string) => ['users', tenantId, 'members', id] as const,
  memberSessions: (tenantId: string, id: string) => ['users', tenantId, 'members', id, 'sessions'] as const,
  invitations: (tenantId: string) => ['users', tenantId, 'invitations'] as const,
  roles: (tenantId: string) => ['users', tenantId, 'roles'] as const,
  permissions: ['permissions'] as const,
}

export const useMembers = (tenantId: string) => useQuery({ queryKey: usersKeys.members(tenantId), queryFn: usersApi.members })
export const useMember = (tenantId: string, id: string) =>
  useQuery({ queryKey: usersKeys.member(tenantId, id), queryFn: () => usersApi.member(id) })
export const useMemberSessions = (tenantId: string, id: string, enabled: boolean) =>
  useQuery({ queryKey: usersKeys.memberSessions(tenantId, id), queryFn: () => usersApi.memberSessions(id), enabled })
export const useInvitations = (tenantId: string) =>
  useQuery({ queryKey: usersKeys.invitations(tenantId), queryFn: usersApi.invitations })
export const useRoles = (tenantId: string) => useQuery({ queryKey: usersKeys.roles(tenantId), queryFn: usersApi.roles })
export const usePermissions = () =>
  useQuery({ queryKey: usersKeys.permissions, queryFn: usersApi.permissions, staleTime: Infinity })
