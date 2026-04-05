import { useEffect, useMemo } from 'react';

const blobUrlCache = new WeakMap<Blob, string>();
const blobUrlReverseCache = new Map<string, Blob>();
const blobUrlRefCount = new Map<string, number>();
const blobUrlRevokeTimers = new Map<string, number>();

export function useBlobObjectUrl(blob: Blob | null | undefined): string | undefined {
  const objectUrl = useMemo(() => {
    if (!blob) return undefined;
    const cached = blobUrlCache.get(blob);
    if (cached) return cached;
    const created = URL.createObjectURL(blob);
    blobUrlCache.set(blob, created);
    blobUrlReverseCache.set(created, blob);
    return created;
  }, [blob]);

  /**
   * Manage the lifecycle of the object URL with reference counting
   * and delayed revocation to prevent issues with URL.revokeObjectURL
   * being called while the URL is still in use by an <img> element,
   * which can cause the image to disappear until it's reloaded.
   */
  useEffect(() => {
    if (!objectUrl) return;
    const pendingRevoke = blobUrlRevokeTimers.get(objectUrl);
    if (pendingRevoke) {
      globalThis.clearTimeout(pendingRevoke);
      blobUrlRevokeTimers.delete(objectUrl);
    }
    const current = blobUrlRefCount.get(objectUrl) ?? 0;
    blobUrlRefCount.set(objectUrl, current + 1);

    return () => {
      const next = (blobUrlRefCount.get(objectUrl) ?? 1) - 1;
      if (next <= 0) {
        blobUrlRefCount.delete(objectUrl);
        const timer = globalThis.setTimeout(() => {
          blobUrlRevokeTimers.delete(objectUrl);
          const cachedBlob = blobUrlReverseCache.get(objectUrl);
          if (cachedBlob) {
            blobUrlCache.delete(cachedBlob);
            blobUrlReverseCache.delete(objectUrl);
          }
          URL.revokeObjectURL(objectUrl);
        }, 200);
        blobUrlRevokeTimers.set(objectUrl, timer);
      } else {
        blobUrlRefCount.set(objectUrl, next);
      }
    };
  }, [objectUrl]);

  return objectUrl;
}
