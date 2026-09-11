import type { Document, NewDocumentMetadata } from '../models/document';
import type { DocumentTypeMetadataType } from '../models/documentTypeMetadataType';
import type { MetadataType } from '../models/metadataType';
import type { MetadataTypesMap } from '../queries/useMetadataTypes';

export type MetadataRow = {
  metadataTypeId: number;
  slug: string;
  name: string;
  value: string;
  dataType: MetadataType['data_type'];
  options: MetadataType['options'] | null;
  required: boolean;
};

export function buildMetadataRows(
  doc: Document,
  metadataTypes: MetadataTypesMap,
  documentTypeMetadataTypes: DocumentTypeMetadataType[],
): MetadataRow[] {
  return documentTypeMetadataTypes.flatMap((dtmdt) => {
    const mdType = metadataTypes[dtmdt.metadata_type_id];
    if (!mdType) return [];

    return {
      metadataTypeId: dtmdt.metadata_type_id,
      slug: mdType.slug,
      name: mdType.name ?? mdType.slug,
      value: doc.metadata?.[mdType.slug] ?? '',
      dataType: mdType.data_type ?? 'string',
      options: mdType.options ?? null,
      required: dtmdt.required,
    };
  }).sort((a, b) => a.name.localeCompare(b.name));
}

export function isMetadataValueSet(value: string | null | undefined) {
  return String(value ?? '').trim().length > 0;
}

export function getMissingRequiredRows(rows: MetadataRow[]): MetadataRow[] {
  return rows.filter((row) => row.required && !isMetadataValueSet(row.value));
}

export function buildMetadataUpdates(originalMetadata: Record<string, string>, rows: MetadataRow[]): NewDocumentMetadata[] {
  return rows
    .filter((row) => (originalMetadata[row.slug] ?? '') !== row.value)
    .map((row) => ({ metadata_type_id: row.metadataTypeId, value: row.value }));
}
