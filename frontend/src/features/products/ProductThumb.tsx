import { useFileUrl } from '../../api/files'
import { Icon } from '../../app/layouts/Icon'
import { useSession } from '../auth/session'

const SIZES = {
  sm: 'size-9 rounded-md',
  md: 'size-11 rounded-lg',
  /** POS product card: sized by the card through `className`. */
  card: 'rounded-md',
} as const

/**
 * A product's photo as a small square for lists (products, POS, purchases),
 * or a neutral tag icon when it has none. The photo is fetched once with the
 * access token and cached for good (a new photo gets a new file id).
 */
export function ProductThumb({
  fileId,
  size = 'sm',
  className = '',
}: {
  fileId: string | null | undefined
  size?: keyof typeof SIZES
  /** Extra sizing classes (POS cards). */
  className?: string
}) {
  const { me } = useSession()
  const url = useFileUrl(me?.tenant?.tenant_id ?? '', fileId)
  return (
    <span className={`flex shrink-0 items-center justify-center overflow-hidden border border-border bg-canvas text-ink-muted ${SIZES[size]} ${className}`}>
      {url ? (
        // Decorative: the product name is always next to it.
        <img src={url} alt="" loading="lazy" decoding="async" className="size-full object-cover" />
      ) : (
        <Icon name="products" className={size === 'card' ? 'size-5 opacity-50' : 'size-4 opacity-60'} />
      )}
    </span>
  )
}
