import { useMemo } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';

import type { ListParams } from '../api';

type ListParamsUpdater<T extends ListParams> = T | ((prev: T) => T);

function parsePositiveIntParam(value: string | null): number | undefined {
  if (!value) return undefined;

  const parsed = Number.parseInt(value, 10);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined;
}

function parseBooleanParam(value: string | null): boolean | undefined {
  if (!value) return undefined;
  if (value === 'true' || value === 'desc') return true;
  if (value === 'false' || value === 'asc') return false;
  return undefined;
}

export function parseListParamsHash<T extends ListParams>(hash: string, defaults: T): T {
  const params = new URLSearchParams(hash.startsWith('#') ? hash.slice(1) : hash);
  const page = parsePositiveIntParam(params.get('page'));
  const perPage = parsePositiveIntParam(params.get('per_page'));
  const searchText = params.get('q')?.trim() || undefined;

  return {
    ...defaults,
    ...(page ? { page } : {}),
    ...(perPage ? { per_page: perPage } : {}),
    ...(searchText ? { q: searchText } : {}),
    ...(params.has('sf') ? { sf: params.get('sf') || undefined } : {}),
    ...(params.has('sd') ? { sd: parseBooleanParam(params.get('sd')) } : {}),
  };
}

export function serializeListParamsHash<T extends ListParams>(params: T, defaults: T): string {
  const urlParams = new URLSearchParams();
  const searchText = params.q?.trim();

  if (params.page && params.page !== defaults.page) {
    urlParams.set('page', String(params.page));
  }
  if (params.per_page && params.per_page !== defaults.per_page) {
    urlParams.set('per_page', String(params.per_page));
  }
  if (searchText) {
    urlParams.set('q', searchText);
  }
  if (params.sf !== defaults.sf) {
    urlParams.set('sf', params.sf ?? '');
  }
  if (params.sd !== defaults.sd) {
    urlParams.set('sd', params.sd ? 'desc' : 'asc');
  }

  return urlParams.toString();
}

export function useHashListParams<T extends ListParams>(defaults: T) {
  const navigate = useNavigate();
  const location = useLocation();
  const listParams = useMemo(() => parseListParamsHash(location.hash, defaults), [defaults, location.hash]);

  const updateListParams = (updater: ListParamsUpdater<T>) => {
    const nextParams = typeof updater === 'function' ? updater(listParams) : updater;

    navigate({
      pathname: location.pathname,
      search: location.search,
      hash: serializeListParamsHash(nextParams, defaults),
    });
  };

  return { listParams, updateListParams };
}
