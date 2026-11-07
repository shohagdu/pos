import { useFileUrl } from '../../api/files'
import { Icon } from '../../app/layouts/Icon'
import { useSession } from '../auth/session'

const SIZES = {
  sm: 'size-9 rounded-md',
  md: 'size-11 rounded-lg',
} as const

/**
 * A product's photo as a small square for lists (products, POS, purchases),
 * or a neutral tag icon when it has none. The photo is fetched once with the
 * access token and cached for good (a new photo gets a new file id).
 */
export function ProductThumb({ fileId, size = 'sm' }: { fileId: string | null | undefined; size?: keyof typeof SIZES }) {
  const { me } = useSession()
  const url = useFileUrl(me?.tenant?.tenant_id ?? '', fileId)
  return (
    <span className={`flex shrink-0 items-center justify-center overflow-hidden border border-border bg-canvas text-ink-muted ${SIZES[size]}`}>
      {url ? (
        // Decorative: the product name is always next to it.
        <img src={url} alt="" loading="lazy" decoding="async" className="size-full object-cover" />
      ) : (
        <Icon name="products" className="size-4 opacity-60" />
      )}
    </span>
  )
}
