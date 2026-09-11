import { describe, expect, it } from 'vitest';

import {
  parseBasicDocumentSearchHash,
  parseDocumentListHash,
  serializeBasicDocumentSearchHash,
  serializeDocumentListHash,
  serializeDocumentListUpdate,
} from '../util/documentListHash';

describe('document list hash helpers', () => {
  it('parses document index value scope from the hash', () => {
    expect(parseDocumentListHash('#document_index_value_id=42')).toMatchObject({
      document_index_value_id: 42,
    });
  });

  it('serializes document index value scope', () => {
    expect(serializeDocumentListHash({ document_index_value_id: 42 })).toBe('sf=&sd=asc&document_index_value_id=42');
  });

  it('preserves scope while applying basic document search', () => {
    expect(serializeBasicDocumentSearchHash('alpha', { document_index_value_id: 42 })).toBe(
      'sf=&sd=asc&document_index_value_id=42&search=alpha',
    );
  });

  it('expands basic search to title and document text only', () => {
    expect(parseDocumentListHash('#search=alpha')).toMatchObject({
      match_any: true,
      q: 'alpha',
      text: 'alpha',
    });
    expect(parseDocumentListHash('#search=alpha')).not.toHaveProperty('metadata_value');
  });

  it('keeps the basic search param when narrowing by cabinet or tag', () => {
    const listParams = parseDocumentListHash('#search=my%20text');
    const narrowed = serializeDocumentListUpdate({ ...listParams, cabinet_id: 2, tag_id: 3, page: 1 }, 'my text');

    expect(narrowed).toContain('search=my+text');
    expect(narrowed).toContain('cabinet_id=2');
    expect(narrowed).toContain('tag_id=3');
    expect(narrowed).not.toContain('q=my');
    expect(narrowed).not.toContain('text=my');

    expect(parseDocumentListHash(`#${narrowed}`)).toMatchObject({
      q: 'my text',
      text: 'my text',
      cabinet_id: 2,
      tag_id: 3,
    });
  });

  it('falls back to title/text params once the basic search text changes', () => {
    const listParams = parseDocumentListHash('#search=my%20text');
    const updated = serializeDocumentListUpdate({ ...listParams, q: 'other', page: 1 }, 'my text');

    expect(updated).toContain('q=other');
    expect(updated).not.toContain('search=');
  });

  it('parses paging, sort, scope, and duplicate flags', () => {
    expect(
      parseDocumentListHash(
        '#page=3&per_page=24&sf=title&sd=desc&cabinet_id=2&tag_id=3&document_index_value_id=7&document_type_id=9&metadata_type_id=11&metadata_value=%20Acme%20&file_content_type=pdf&duplicates=true&duplicate_checksum=true',
      ),
    ).toMatchObject({
      page: 3,
      per_page: 24,
      sf: 'title',
      sd: true,
      cabinet_id: 2,
      tag_id: 3,
      document_index_value_id: 7,
      document_type_id: 9,
      metadata_type_id: 11,
      metadata_value: 'Acme',
      file_content_type: 'pdf',
      duplicates: true,
      duplicate_checksum: true,
    });
  });

  it('ignores invalid paging and sort-direction values', () => {
    const parsed = parseDocumentListHash('#page=0&per_page=-5&sd=bogus');

    expect(parsed.page).toBe(1);
    expect(parsed.per_page).toBe(12);
    expect(parsed.sd).toBeUndefined();
  });

  it('rejects unsupported page sizes and falls back to the default', () => {
    expect(parseDocumentListHash('#per_page=7').per_page).toBe(12);
    expect(parseDocumentListHash('#per_page=48').per_page).toBe(48);
  });

  it('parses ascending sort aliases', () => {
    expect(parseDocumentListHash('#sd=asc').sd).toBe(false);
    expect(parseDocumentListHash('#sd=false').sd).toBe(false);
  });

  it('keeps narrowing filters alongside basic search', () => {
    expect(parseDocumentListHash('#search=alpha&cabinet_id=2&tag_id=3&document_type_id=4')).toMatchObject({
      q: 'alpha',
      text: 'alpha',
      match_any: true,
      cabinet_id: 2,
      tag_id: 3,
      document_type_id: 4,
    });
  });

  it('trims basic search text and ignores blank search', () => {
    expect(parseBasicDocumentSearchHash('#search=%20%20invoice%20%20')).toBe('invoice');
    expect(parseBasicDocumentSearchHash('#q=invoice')).toBe('');
    expect(parseDocumentListHash('#search=%20%20')).not.toHaveProperty('q');
  });

  it('serializes only non-default paging and sort values', () => {
    expect(serializeDocumentListHash({ page: 1, per_page: 12, sf: 'created_at', sd: true })).toBe('');
    expect(serializeDocumentListHash({ page: 2, per_page: 24, sf: 'title', sd: false })).toBe(
      'page=2&per_page=24&sf=title&sd=asc',
    );
  });

  it('trims text-search values when serializing', () => {
    const hash = serializeDocumentListHash({ q: '  invoice  ', text: '  receipt  ' });

    expect(hash).toContain('q=invoice');
    expect(hash).toContain('text=receipt');
  });

  it('resets to the first page and preserves narrowing filters for basic search', () => {
    const hash = serializeBasicDocumentSearchHash('  alpha  ', {
      page: 4,
      document_type_id: 2,
      metadata_value: ' Acme ',
      metadata_type_id: 3,
      file_content_type: 'pdf',
      cabinet_id: 5,
      tag_id: 6,
      document_index_value_id: 7,
      duplicates: true,
    });

    expect(hash).toContain('search=alpha');
    expect(hash).toContain('document_type_id=2');
    expect(hash).toContain('metadata_value=Acme');
    expect(hash).toContain('metadata_type_id=3');
    expect(hash).toContain('file_content_type=pdf');
    expect(hash).toContain('cabinet_id=5');
    expect(hash).toContain('tag_id=6');
    expect(hash).toContain('document_index_value_id=7');
    expect(hash).toContain('duplicates=true');
    expect(hash).not.toContain('page=4');
    expect(hash).not.toContain('q=alpha');
    expect(hash).not.toContain('match_any');
  });

  it('does not preserve basic search without an exact title/text/match-any triple', () => {
    const base = parseDocumentListHash('#search=alpha');

    expect(serializeDocumentListUpdate({ ...base, match_any: undefined }, 'alpha')).not.toContain('search=');
    expect(serializeDocumentListUpdate({ ...base, text: 'beta' }, 'alpha')).not.toContain('search=');
    expect(serializeDocumentListUpdate({ ...base }, '')).not.toContain('search=');
  });

  it('round-trips a scoped basic search through narrowing and parsing', () => {
    const narrowed = serializeDocumentListUpdate(
      {
        ...parseDocumentListHash('#search=invoice&document_index_value_id=11'),
        cabinet_id: 2,
        page: 1,
      },
      'invoice',
    );

    expect(parseDocumentListHash(`#${narrowed}`)).toMatchObject({
      q: 'invoice',
      text: 'invoice',
      match_any: true,
      cabinet_id: 2,
      document_index_value_id: 11,
    });
  });
});
