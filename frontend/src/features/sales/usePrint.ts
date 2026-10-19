import { useCallback, useEffect, useState } from 'react'

/**
 * Asks the browser to print after the next render, so the print-only
 * receipt shows the latest data (e.g. "REPRINT n" after a reprint).
 */
export function usePrint(): () => void {
  const [request, setRequest] = useState(0)
  useEffect(() => {
    if (request > 0) window.print()
  }, [request])
  return useCallback(() => setRequest((n) => n + 1), [])
}
