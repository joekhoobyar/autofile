import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ListDocuments } from './documents';

const mockUseDocuments = vi.fn();
const mockDocumentActions = vi.fn();

vi.mock('../components/DocumentActions', () => ({
  DocumentActions: (props: unknown) => {
    mockDocumentActions(props);
    return null;
  },
}));

vi.mock('../queries/useDocuments', () => ({
  useDocuments: (params: unknown) => mockUseDocuments(params),
  useDocumentThumbnail: () => ({ data: undefined }),
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

function documentRow(id: number, title: string) {
  return {
    id,
    title,
    document_type_id: 1,
    metadata: {},
    pages: 1,
    created_at: '2024-01-01T00:00:00Z',
    created_by: 1,
    updated_at: '2024-01-01T00:00:00Z',
    updated_by: 1,
    cabinet_ids: [],
    tag_ids: [],
  };
}

describe('ListDocuments search scope', () => {
  beforeEach(() => {
    mockUseDocuments.mockReset();
    mockDocumentActions.mockReset();
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

  it('clearing basic search preserves narrowing filters', async () => {
    renderList('/documents#search=invoice&document_type_id=2', '/documents');

    fireEvent.click(screen.getByLabelText('Clear search'));

    await waitFor(() => {
      expect(mockUseDocuments).toHaveBeenLastCalledWith(
        expect.objectContaining({ document_type_id: 2 }),
      );
    });
    expect(mockUseDocuments).toHaveBeenLastCalledWith(
      expect.not.objectContaining({ q: expect.anything(), text: expect.anything() }),
    );
  });

  it('selects all visible documents', () => {
    mockUseDocuments.mockReturnValue({
      isPending: false,
      isFetching: false,
      data: { items: [documentRow(1, 'Alpha'), documentRow(2, 'Beta')], page: 1, per_page: 12, total: 2 },
    });
    renderList('/documents', '/documents');

    const selectAll = screen.getAllByLabelText('Select all documents on this page');
    const selectAllInput = selectAll.find((el) => el.tagName === 'INPUT') ?? selectAll[0];
    fireEvent.click(selectAllInput);

    expect(mockDocumentActions).toHaveBeenLastCalledWith(expect.objectContaining({ documentIds: [1, 2] }));
  });
});
