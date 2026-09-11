import { describe, expect, it } from 'vitest';

import {
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
});
