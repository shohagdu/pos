import type { PaymentStatus, RefundMode, SaleStatus } from '../../api/sales'

export const SALE_STATUSES: SaleStatus[] = ['completed', 'partially_returned', 'returned', 'cancelled']
export const PAYMENT_STATUSES: PaymentStatus[] = ['paid', 'partial', 'unpaid', 'refunded']
export const REFUND_MODES: RefundMode[] = ['cash', 'original_method', 'customer_credit']
