import type { Supplier, SupplierInput } from '../../api/parties'

/** The full update body for a supplier (PUT replaces every field). */
export function supplierInput(s: Supplier, patch: Partial<SupplierInput> = {}): SupplierInput {
  return {
    name: s.name,
    contact_person: s.contact_person ?? undefined,
    phone: s.phone ?? undefined,
    email: s.email ?? undefined,
    address: s.address ?? undefined,
    notes: s.notes ?? undefined,
    status: s.status,
    ...patch,
  }
}
