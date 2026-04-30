import { apiFetchRaw } from '../api';

export const SLUG_PATTERN = /^[a-z0-9_-]+$/;

export function createSlugRules(slugEndpoint: string, validateUnique = true) {
  let timeout: ReturnType<typeof setTimeout> | undefined;
  let requestId = 0;
  let resolvePending: ((value: true | string) => void) | undefined;

  return {
    required: 'Slug is required',
    minLength: { value: 2, message: 'Slug must be at least 2 characters' },
    pattern: {
      value: SLUG_PATTERN,
      message: 'Slug may only contain lowercase letters, numbers, hyphens, and underscores',
    },
    validate: {
      unique: (value?: string) => {
        if (!validateUnique || !value || !SLUG_PATTERN.test(value)) {
          return true;
        }

        if (timeout) {
          clearTimeout(timeout);
        }
        resolvePending?.(true);

        const currentRequestId = ++requestId;

        return new Promise<true | string>((resolve) => {
          resolvePending = resolve;
          timeout = setTimeout(async () => {
            resolvePending = undefined;

            try {
              const response = await apiFetchRaw(`${slugEndpoint}/${encodeURIComponent(value)}`);
              if (currentRequestId !== requestId) {
                resolve(true);
                return;
              }

              if (response.status === 404) {
                resolve(true);
                return;
              }

              if (response.ok) {
                resolve('Slug is already in use');
                return;
              }

              resolve('Unable to verify slug uniqueness');
            } catch {
              resolve('Unable to verify slug uniqueness');
            }
          }, 300);
        });
      },
    },
  };
}

export function normalizeSlug(value: string): string {
  return value.toLowerCase();
}
