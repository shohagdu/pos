/** What the API accepts for product photos (it re-checks the whole file). */
export const IMAGE_TYPES = ['image/jpeg', 'image/png', 'image/webp']
export const MAX_IMAGE_BYTES = 2 * 1024 * 1024

/** The format from the first bytes; the declared type and file name are not trusted. */
function sniff(b: Uint8Array): string | null {
  if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return 'image/jpeg'
  if ([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a].every((x, i) => b[i] === x)) return 'image/png'
  const ascii = (from: number, to: number) => String.fromCharCode(...b.subarray(from, to))
  if (ascii(0, 4) === 'RIFF' && ascii(8, 12) === 'WEBP') return 'image/webp'
  return null
}

// Same rules as the server (`pos_files::image_check`): tags only count when a
// space, `>` or `/` follows, so random image data does not trip them.
const UNSAFE_TAG = /<\/?(script|html|body|head|svg|iframe|object|embed|form|meta|link|style|img)[\s>/]/i
const UNSAFE_TEXT = /<\?php|javascript:|vbscript:|onerror=|onload=/i

/** Markup or script hidden in the bytes (latin1 keeps one char per byte). */
export function hasMarkup(bytes: Uint8Array): boolean {
  let text = ''
  for (let i = 0; i < bytes.length; i += 0x8000) text += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
  return UNSAFE_TAG.test(text) || UNSAFE_TEXT.test(text)
}

async function readBytes(file: Blob): Promise<Uint8Array> {
  if (typeof file.arrayBuffer === 'function') return new Uint8Array(await file.arrayBuffer())
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer))
    reader.onerror = () => reject(reader.error ?? new Error('read failed'))
    reader.readAsArrayBuffer(file)
  })
}

/**
 * A translation key when the file cannot be used, otherwise `null`. Checks the
 * type, the size, the real format from the bytes and hidden markup; the server
 * also checks the file's structure from start to end.
 */
export async function imageProblem(file: File): Promise<string | null> {
  if (!IMAGE_TYPES.includes(file.type)) return 'products.image.wrongType'
  if (file.size > MAX_IMAGE_BYTES) return 'products.image.tooLarge'
  const bytes = await readBytes(file)
  if (sniff(bytes) === null) return 'products.image.notImage'
  if (hasMarkup(bytes)) return 'products.image.unsafe'
  return null
}
