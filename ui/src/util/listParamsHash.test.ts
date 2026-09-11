import { describe, expect, it } from 'vitest';

import type { ListParams } from '../api';
import { parseListParamsHash, serializeListParamsHash } from './listParamsHash';

const defaults: ListParams = {
  page: 1,
  per_page: 25,
  sf: 'name',
  sd: false,
  include_deleted: false,
};

describe('parseListParamsHash', () => {
  it('parses paging, search, sort, and include_deleted values', () => {
    expect(parseListParamsHash('#page=2&per_page=50&q=%20invoice%20&sf=created_at&sd=desc&include_deleted=true', defaults)).toEqual({
      ...defaults,
      page: 2,
      per_page: 50,
      q: 'invoice',
      sf: 'created_at',
      sd: true,
      include_deleted: true,
    });
  });

  it('ignores invalid positive integer values', () => {
    expect(parseListParamsHash('#page=0&per_page=-10&q=%20%20', defaults)).toEqual(defaults);
  });

  it('parses ascending sort direction aliases', () => {
    expect(parseListParamsHash('#sd=asc&include_deleted=false', { ...defaults, sd: true, include_deleted: true })).toEqual({
      ...defaults,
      sd: false,
      include_deleted: false,
    });
  });
});

describe('serializeListParamsHash', () => {
  it('serializes only values that differ from defaults', () => {
    expect(serializeListParamsHash({ ...defaults, page: 3, q: 'receipt', sd: true }, defaults)).toBe('page=3&q=receipt&sd=desc');
  });

  it('serializes explicit ascending sort and include_deleted false when they differ from defaults', () => {
    expect(serializeListParamsHash({ ...defaults, sd: false, include_deleted: false }, { ...defaults, sd: true, include_deleted: true })).toBe(
      'sd=asc&include_deleted=false',
    );
  });
});
