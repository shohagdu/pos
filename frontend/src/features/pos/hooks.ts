import { useEffect, useState } from 'react'

/** `value` after it stopped changing for `ms`. */
export function useDebounced<T>(value: T, ms = 200): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), ms)
    return () => clearTimeout(timer)
  }, [value, ms])
  return debounced
}

export { useMediaQuery } from '../../lib/useMediaQuery'

/** Tracks `navigator.onLine` (offline banner). */
export function useOnline(): boolean {
  const [online, setOnline] = useState(() => (typeof navigator === 'undefined' ? true : navigator.onLine))
  useEffect(() => {
    const on = () => setOnline(true)
    const off = () => setOnline(false)
    window.addEventListener('online', on)
    window.addEventListener('offline', off)
    return () => {
      window.removeEventListener('online', on)
      window.removeEventListener('offline', off)
    }
  }, [])
  return online
}

/**
 * The business's local time now (`YYYY-MM-DDTHH:MM:SS`), from the server's
 * clock (`PosContext.local_now`) plus the time since it was read, ticking
 * every 15 s: promotion schedules follow the shop's clock, not the device's.
 */
export function useLocalClock(serverNow: string | undefined): string | null {
  const [ms, setMs] = useState<number | null>(null)
  useEffect(() => {
    if (!serverNow) return
    const [date, time = '00:00:00'] = serverNow.split('T')
    const [y, m, d] = (date ?? '').split('-').map(Number)
    const [hh, mm, ss] = time.split(':').map((v) => Number.parseInt(v, 10))
    const start = Date.UTC(y!, m! - 1, d!, hh ?? 0, mm ?? 0, ss ?? 0)
    const at = Date.now()
    const update = () => setMs(start + Date.now() - at)
    const first = setTimeout(update, 0)
    const id = setInterval(update, 15_000)
    return () => {
      clearTimeout(first)
      clearInterval(id)
    }
  }, [serverNow])
  if (ms === null) return serverNow?.slice(0, 19) ?? null
  return new Date(ms).toISOString().slice(0, 19)
}
