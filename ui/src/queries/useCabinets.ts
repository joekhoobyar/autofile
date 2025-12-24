import { useQuery, type UseQueryResult } from '@tanstack/react-query';
import { apiFetch } from '../api';
import { type Cabinet } from '../models/cabinet';

export function useCabinets(): UseQueryResult<Cabinet[], Error> {
  return useQuery({
    queryKey: ['cabinets', 'list'],
    queryFn: () => apiFetch<Cabinet[]>('cabinets'),
  });
}
