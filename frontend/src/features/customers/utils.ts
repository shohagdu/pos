import type { Customer, CustomerInput } from '../../api/parties'

/** The full update body for a customer, with changes (PUT replaces every field). */
export function customerInput(c: Customer, patch: Partial<CustomerInput> = {}): CustomerInput {
  return {
    name: c.name,
    phone: c.phone ?? undefined,
    email: c.email ?? undefined,
    address: c.address ?? undefined,
    credit_limit: c.credit_limit ?? undefined,
    notes: c.notes ?? undefined,
    status: c.status,
    ...patch,
  }
}
