import { describe, expect, it } from 'vitest';

import type { Document } from '../models/document';
import type { DocumentTypeMetadataType } from '../models/documentTypeMetadataType';
import type { MetadataType } from '../models/metadataType';
import type { MetadataTypesMap } from '../queries/useMetadataTypes';
import { buildMetadataRows, buildMetadataUpdates, getMissingRequiredRows, isMetadataValueSet, type MetadataRow } from './documentMetadataRows';

const doc: Document = {
  id: 10,
  title: 'Invoice 100',
  document_type_id: 2,
  metadata: {
    vendor: 'Acme',
    invoice_date: '2024-01-02',
  },
  pages: 1,
  created_at: '2024-01-01T00:00:00Z',
  created_by: 1,
  updated_at: '2024-01-01T00:00:00Z',
  updated_by: 1,
};

const metadataTypes: MetadataTypesMap = {
  1: metadataType({ id: 1, slug: 'vendor', name: 'Vendor', data_type: 'string' }),
  2: metadataType({ id: 2, slug: 'invoice_date', name: 'Invoice Date', data_type: 'date' }),
  3: metadataType({ id: 3, slug: 'status', name: 'Status', data_type: 'lookup', options: { choices: ['Open', 'Paid'] } }),
};

const documentTypeMetadataTypes: DocumentTypeMetadataType[] = [
  { document_type_id: 2, metadata_type_id: 1, required: true },
  { document_type_id: 2, metadata_type_id: 3, required: false },
  { document_type_id: 2, metadata_type_id: 2, required: true },
  { document_type_id: 2, metadata_type_id: 999, required: true },
];

function metadataType(input: Omit<MetadataType, 'created_at' | 'updated_at'>): MetadataType {
  return {
    ...input,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  };
}

function row(input: Partial<MetadataRow> & Pick<MetadataRow, 'metadataTypeId' | 'slug' | 'value'>): MetadataRow {
  return {
    name: input.slug,
    dataType: 'string',
    options: null,
    required: false,
    ...input,
  };
}

describe('buildMetadataRows', () => {
  it('builds sorted metadata rows from document type fields and current document metadata', () => {
    expect(buildMetadataRows(doc, metadataTypes, documentTypeMetadataTypes)).toEqual([
      {
        metadataTypeId: 2,
        slug: 'invoice_date',
        name: 'Invoice Date',
        value: '2024-01-02',
        dataType: 'date',
        options: null,
        required: true,
      },
      {
        metadataTypeId: 3,
        slug: 'status',
        name: 'Status',
        value: '',
        dataType: 'lookup',
        options: { choices: ['Open', 'Paid'] },
        required: false,
      },
      {
        metadataTypeId: 1,
        slug: 'vendor',
        name: 'Vendor',
        value: 'Acme',
        dataType: 'string',
        options: null,
        required: true,
      },
    ]);
  });
});

describe('required metadata detection', () => {
  it('treats trimmed non-empty strings as set', () => {
    expect(isMetadataValueSet(' value ')).toBe(true);
    expect(isMetadataValueSet('   ')).toBe(false);
    expect(isMetadataValueSet(null)).toBe(false);
  });

  it('returns required rows without values', () => {
    const rows = [
      row({ metadataTypeId: 1, slug: 'vendor', value: 'Acme', required: true }),
      row({ metadataTypeId: 2, slug: 'invoice_date', value: ' ', required: true }),
      row({ metadataTypeId: 3, slug: 'status', value: '', required: false }),
    ];

    expect(getMissingRequiredRows(rows)).toEqual([rows[1]]);
  });
});

describe('buildMetadataUpdates', () => {
  it('builds a save payload with only changed values', () => {
    const rows = [
      row({ metadataTypeId: 1, slug: 'vendor', value: 'Acme' }),
      row({ metadataTypeId: 2, slug: 'invoice_date', value: '2024-01-03' }),
      row({ metadataTypeId: 3, slug: 'status', value: 'Paid' }),
    ];

    expect(buildMetadataUpdates(doc.metadata, rows)).toEqual([
      { metadata_type_id: 2, value: '2024-01-03' },
      { metadata_type_id: 3, value: 'Paid' },
    ]);
  });
});
