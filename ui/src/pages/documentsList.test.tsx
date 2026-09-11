import { cleanup, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ListDocuments } from './documents';

const mockUseDocuments = vi.fn();

vi.mock('../components/DocumentActions', () => ({
  DocumentActions: () => null,
}));

vi.mock('../queries/useDocuments', () => ({
  useDocuments: (params: unknown) => mockUseDocuments(params),
}));

vi.mock('../queries/useDocumentIndexes', () => ({
  useDocumentIndex: () => ({ data: undefined }),
}));

vi.mock('../queries/useDocumentIndexValues', () => ({
  useDocumentIndexValueAncestors: () => ({ data: [] }),
}));

vi.mock('../queries/useCabinets', () => ({
  useCabinets: () => ({ data: { items: [] } }),
  useCabinetTree: () => ({ data: [], isPending: false, isFetching: false }),
}));

vi.mock('../queries/useTags', () => ({
  useTags: () => ({ data: { items: [] }, isPending: false, isFetching: false }),
}));

vi.mock('../queries/useDocumentTypes', () => ({
  useDocumentTypes: () => ({ data: { items: [] }, isPending: false, isFetching: false }),
  useDocumentTypesMap: () => ({ data: {} }),
}));

vi.mock('../queries/useMetadataTypes', () => ({
  useMetadataTypes: () => ({ data: { items: [] }, isPending: false, isFetching: false }),
  useMetadataTypesMap: () => ({ data: {} }),
}));

function renderList(path: string, routePath: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path={routePath} element={<ListDocuments />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('ListDocuments search scope', () => {
  beforeEach(() => {
    mockUseDocuments.mockReset();
    mockUseDocuments.mockReturnValue({ isPending: false, isFetching: false, data: { items: [] } });
  });

  afterEach(() => {
    cleanup();
  });

  it('combines cabinet route scope with basic title/text search', () => {
    renderList('/cabinets/2/documents#search=invoice', '/cabinets/:cabinetId/documents');

    expect(mockUseDocuments).toHaveBeenCalledWith(
      expect.objectContaining({
        cabinet_id: 2,
        match_any: true,
        q: 'invoice',
        text: 'invoice',
      }),
    );
  });

  it('combines tag route scope with basic title/text search', () => {
    renderList('/tags/3/documents#search=invoice', '/tags/:tagId/documents');

    expect(mockUseDocuments).toHaveBeenCalledWith(
      expect.objectContaining({
        tag_id: 3,
        match_any: true,
        q: 'invoice',
        text: 'invoice',
      }),
    );
  });

  it('combines index-value route scope with basic title/text search', () => {
    renderList('/indexes/4/values/11/documents#search=invoice', '/indexes/:documentIndexId/values/:documentIndexValueId/documents');

    expect(mockUseDocuments).toHaveBeenCalledWith(
      expect.objectContaining({
        document_index_value_id: 11,
        match_any: true,
        q: 'invoice',
        text: 'invoice',
      }),
    );
  });

  it('shows a single Search chip for basic search instead of Title/Text chips', () => {
    renderList('/documents#search=my+text', '/documents');

    expect(screen.getAllByText('Search: my text').length).toBeGreaterThan(0);
    expect(screen.queryByText('Title: my text')).not.toBeInTheDocument();
    expect(screen.queryByText('Text: my text')).not.toBeInTheDocument();
  });

  it('shows separate Title and Text chips for advanced search', () => {
    renderList('/documents#q=invoice&text=receipt', '/documents');

    expect(screen.getAllByText('Title: invoice').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Text: receipt').length).toBeGreaterThan(0);
    expect(screen.queryByText(/^Search:/)).not.toBeInTheDocument();
  });
});
