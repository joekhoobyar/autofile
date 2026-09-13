import { describe, expect, it } from 'vitest';

import { getPasswordStrength } from './passwordStrength';

describe('getPasswordStrength', () => {
  it('returns empty for missing values', () => {
    expect(getPasswordStrength('')).toBe('empty');
    expect(getPasswordStrength(null)).toBe('empty');
    expect(getPasswordStrength(undefined)).toBe('empty');
  });

  it('returns weak for short or low-variety passwords', () => {
    expect(getPasswordStrength('short')).toBe('weak');
    expect(getPasswordStrength('aaaaaaaaaaaa')).toBe('weak');
    expect(getPasswordStrength('Abc')).toBe('weak');
  });

  it('returns medium for 12+ characters with two character classes', () => {
    expect(getPasswordStrength('Abcdefghijkl')).toBe('medium');
    expect(getPasswordStrength('abc123456789')).toBe('medium');
    expect(getPasswordStrength('Abc123456789')).toBe('medium');
  });

  it('returns strong for 12+ characters with upper, lower, digit, and symbol', () => {
    expect(getPasswordStrength('Abc123456789!')).toBe('strong');
  });

  it('never rates a sub-12-character password medium or strong', () => {
    expect(getPasswordStrength('Ab1!')).toBe('weak');
    expect(getPasswordStrength('Abc123!')).toBe('weak');
  });
});
