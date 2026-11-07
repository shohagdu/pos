import { useQuery } from '@tanstack/react-query'
import { useEffect, useState } from 'react'

import { apiFetch } from './client'

/** Uploaded files are read with the access token, so `<img src>` cannot fetch them directly. */
export const filesApi = {
  get: (id: string) => apiFetch<Blob>(`/api/v1/files/${id}`, { responseType: 'blob' }),
}

/**
 * An object URL for a stored file, or `null` while loading / without one.
 * Files never change (a new upload gets a new id), so the bytes are cached for good.
 */
export function useFileUrl(tenantId: string, fileId: string | null | undefined): string | null {
  const file = useQuery({
    queryKey: ['files', tenantId, fileId] as const,
    queryFn: () => filesApi.get(fileId ?? ''),
    enabled: Boolean(fileId),
    staleTime: Infinity,
  })
  return useObjectUrl(fileId ? file.data : undefined)
}

/** An object URL for a blob, revoked when the blob changes or the component goes away. */
export function useObjectUrl(blob: Blob | null | undefined): string | null {
  const [url, setUrl] = useState<string | null>(null)
  useEffect(() => {
    if (!blob) {
      setUrl(null)
      return
    }
    const next = URL.createObjectURL(blob)
    setUrl(next)
    return () => URL.revokeObjectURL(next)
  }, [blob])
  return url
}
