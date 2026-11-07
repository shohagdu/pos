import type { PaymentAccount } from '../api/money'

/** A balance as a plain number with two decimals and the language's digits: 180.00, ১৮০.০০. */
export function formatBalance(amount: string, lang = 'en'): string {
  const locale = lang === 'bn' ? 'bn-BD' : lang === 'it' ? 'it-IT' : 'en-IN'
  return new Intl.NumberFormat(locale, { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(amount as unknown as number)
}

/**
 * An account in a picker: `Cash (180.00)`. The balance is left out when the
 * member cannot see balances (the API sends 0 to members without `accounts.view`).
 */
export function accountLabel(account: PaymentAccount, lang: string, showBalance: boolean): string {
  return showBalance ? `${account.name} (${formatBalance(account.current_balance, lang)})` : account.name
}
