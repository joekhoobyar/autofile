export const SLUG_PATTERN = /^[a-z0-9_-]+$/;

export const slugRules = {
  required: 'Slug is required',
  minLength: { value: 2, message: 'Slug must be at least 2 characters' },
  pattern: {
    value: SLUG_PATTERN,
    message: 'Slug may only contain lowercase letters, numbers, hyphens, and underscores',
  },
};

export function normalizeSlug(value: string): string {
  return value.toLowerCase();
}
