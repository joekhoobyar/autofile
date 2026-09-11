import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { EditDocumentMetadata } from './documentMetadata';

const mutateAsync = vi.fn();
let documentMetadata: Record<string, string>;

vi.mock('../components/DocumentViewLayout', () => ({
  DocumentViewLayout: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock('../queries/useDocuments', () => ({
  useDocument: () => ({
    isLoading: false,
    isError: false,
    data: {
      id: 10,
      title: 'Invoice 100',
      document_type_id: 2,
      metadata: documentMetadata,
      pages: 1,
      created_at: '2024-01-01T00:00:00Z',
      created_by: 1,
      updated_at: '2024-01-01T00:00:00Z',
      updated_by: 1,
    },
  }),
  useSaveDocumentMetadata: () => ({
    mutateAsync,
    isPending: false,
    isError: false,
  }),
}));

vi.mock('../queries/useMetadataTypes', () => ({
  fetchMetadataTypeValues: vi.fn(() => Promise.resolve([])),
  useDocumentTypeMetadataTypes: () => ({
    isLoading: false,
    data: [
      { document_type_id: 2, metadata_type_id: 1, required: true },
      { document_type_id: 2, metadata_type_id: 2, required: false },
    ],
  }),
  useMetadataTypesMap: () => ({
    isLoading: false,
    data: {
      1: {
        id: 1,
        slug: 'vendor',
        name: 'Vendor',
        data_type: 'string',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      },
      2: {
        id: 2,
        slug: 'invoice_date',
        name: 'Invoice Date',
        data_type: 'date',
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      },
    },
  }),
}));

function renderPage() {
  return render(
    <MemoryRouter initialEntries={['/documents/10/metadata']}>
      <Routes>
        <Route path="/documents/:id/metadata" element={<EditDocumentMetadata />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('EditDocumentMetadata', () => {
  beforeEach(() => {
    documentMetadata = {
      vendor: '',
      invoice_date: '2024-01-02',
    };
    mutateAsync.mockReset();
    mutateAsync.mockResolvedValue([]);
  });

  afterEach(() => {
    cleanup();
  });

  it('shows missing required metadata and disables save', () => {
    renderPage();

    expect(screen.getByText('Document Metadata: Invoice 100')).toBeInTheDocument();
    expect(screen.getByText('Required fields are missing: Vendor')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /save/i })).toBeDisabled();
  });

  it('commits a text cell edit on blur before saving', async () => {
    documentMetadata = {
      vendor: 'Acme',
      invoice_date: '2024-01-02',
    };
    renderPage();

    fireEvent.click(screen.getByText('Acme'));
    fireEvent.doubleClick(screen.getByText('Acme'));

    const input = await screen.findByDisplayValue('Acme');
    fireEvent.input(input, { target: { value: 'Globex' } });
    expect(await screen.findByDisplayValue('Globex')).toBeInTheDocument();
    fireEvent.blur(input);

    fireEvent.click(screen.getByRole('button', { name: /save/i }));

    await waitFor(() => {
      expect(mutateAsync).toHaveBeenCalledWith([{ metadata_type_id: 1, value: 'Globex' }]);
    });
  });
});
