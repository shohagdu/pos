import { useEffect, useState } from 'react'

/**
 * Whether a CSS media query matches, kept up to date. Where `matchMedia` is
 * missing (tests, very old browsers) it answers `fallback`.
 */
export function useMediaQuery(query: string, fallback = false): boolean {
  const supported = typeof window !== 'undefined' && typeof window.matchMedia === 'function'
  const [matches, setMatches] = useState(() => (supported ? window.matchMedia(query).matches : fallback))
  useEffect(() => {
    if (!supported) return
    const mq = window.matchMedia(query)
    const on = () => setMatches(mq.matches)
    on()
    mq.addEventListener('change', on)
    return () => mq.removeEventListener('change', on)
  }, [query, supported])
  return matches
}
