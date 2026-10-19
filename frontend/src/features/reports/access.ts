/** Who may open which report (mirrors the API's permission checks). */
export function reportAccess(can: (permission: string) => boolean) {
  const financial = can('reports.financial')
  const statements = financial || can('accounting.view')
  return {
    sales: can('reports.sales'),
    purchases: can('reports.purchases'),
    inventory: can('reports.inventory'),
    expenses: financial || can('expenses.view'),
    payments: financial,
    profit: financial,
    statements,
    journals: can('accounting.view'),
    parties: can('reports.parties'),
    vat: financial,
    discounts: can('reports.sales'),
    movement: can('reports.inventory'),
  }
}

export type ReportAccess = ReturnType<typeof reportAccess>
