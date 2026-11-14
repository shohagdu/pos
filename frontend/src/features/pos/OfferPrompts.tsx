import { useMutation } from '@tanstack/react-query'
import { type Dispatch, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { catalogApi } from '../../api/catalog'
import { type PosItem, salesApi } from '../../api/sales'
import { dec, toPlain } from '../../lib/decimal'
import { formatQuantity } from '../../lib/format'
import type { Offer } from '../../lib/promotions'
import type { CartAction, CartLine } from './cart'

/**
 * Buy X Get Y: when the cart qualifies for a free item it does not hold yet,
 * ask before adding it (§4.3: never add stock silently). The same product is
 * topped up on its line; another product is looked up and added; for a
 * category the cashier picks the item.
 */
export function OfferPrompts({
  offers,
  lines,
  dispatch,
  onAdd,
}: {
  offers: Offer[]
  lines: CartLine[]
  dispatch: Dispatch<CartAction>
  onAdd: (item: PosItem) => void
}) {
  const { t, i18n } = useTranslation()
  const lang = i18n.language
  const [dismissed, setDismissed] = useState<string[]>([])
  const addProduct = useMutation({
    // A product's default variant, as the POS sells it (price, stock).
    mutationFn: async (offer: Offer) => {
      const product = await catalogApi.product(offer.targetId)
      const found = await salesApi.posProducts(product.sku, 5)
      const item = found.find((f) => f.variant_id === product.variant_id)
      if (!item) throw new Error('not found')
      return { item, quantity: offer.quantity }
    },
    onSuccess: ({ item, quantity }) => {
      onAdd(item)
      const q = dec(toPlain(quantity))
      if (q > dec(1)) dispatch({ type: 'quantity', variantId: item.variant_id, quantity: toPlain(q) })
    },
  })

  const key = (o: Offer) => `${o.promotion.id}:${o.targetId}:${toPlain(o.quantity)}`
  const shown = offers.filter((o) => !dismissed.includes(key(o)))
  if (shown.length === 0) return null

  return (
    <ul className="flex flex-col gap-1.5" aria-label={t('pos.offerPromptsLabel')}>
      {shown.map((o) => {
        const promoName = lang === 'bn' && o.promotion.name_bn ? o.promotion.name_bn : o.promotion.name
        const line = o.getTargetType === 'same_product' ? lines.find((l) => l.item.variant_id === o.targetId) : undefined
        const itemName = line ? (lang === 'bn' && line.item.name_bn ? line.item.name_bn : line.item.name) : null
        const qty = formatQuantity(toPlain(o.quantity), lang, true)
        const text =
          o.getTargetType === 'category'
            ? t('pos.offerPromptCategory', { promo: promoName, qty })
            : itemName
              ? t('pos.offerPrompt', { promo: promoName, qty, name: itemName })
              : t('pos.offerPromptProduct', { promo: promoName, qty })
        return (
          <li key={key(o)} className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-brand-500 bg-brand-50 px-3 py-2 text-sm">
            <span className="min-w-0">
              <span aria-hidden="true">🎁 </span>
              {text}
            </span>
            <span className="flex shrink-0 gap-1">
              {o.getTargetType !== 'category' && (
                <button
                  type="button"
                  className="min-h-9 rounded-lg bg-brand-600 px-3 font-semibold text-white hover:bg-brand-700 disabled:opacity-60"
                  disabled={addProduct.isPending}
                  onClick={() => {
                    if (line) {
                      dispatch({ type: 'quantity', variantId: line.item.variant_id, quantity: toPlain(dec(line.quantity) + dec(toPlain(o.quantity))) })
                    } else {
                      addProduct.mutate(o)
                    }
                  }}
                >
                  {t('pos.addFree')}
                </button>
              )}
              <button
                type="button"
                className="min-h-9 rounded-lg px-3 font-medium text-ink-muted hover:bg-canvas"
                onClick={() => setDismissed((d) => [...d, key(o)])}
              >
                {t('pos.noThanks')}
              </button>
            </span>
          </li>
        )
      })}
      {addProduct.isError && <li className="text-xs text-danger">{t('pos.offerNotAvailable')}</li>}
    </ul>
  )
}
