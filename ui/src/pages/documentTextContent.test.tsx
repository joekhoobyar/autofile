import { cleanup, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ListDocumentFilePageTextContent } from './documentFiles';

const mockUseDocument = vi.fn();
const mockUseDocumentFiles = vi.fn();
const mockUseDocumentFilePages = vi.fn();

vi.mock('../components/DocumentViewLayout', () => ({
  DocumentViewLayout: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock('../queries/useDocuments', () => ({
  useDocument: (id: unknown, options?: unknown) => mockUseDocument(id, options),
}));

vi.mock('../queries/useDocumentFiles', () => ({
  useDocumentFiles: (id: unknown) => mockUseDocumentFiles(id),
  useDocumentFilePages: (documentId: unknown, fileId: unknown, options?: unknown) =>
    mockUseDocumentFilePages(documentId, fileId, options),
}));

function documentFixture() {
  return {
    id: 10,
    title: 'Invoice 100',
    document_type_id: 1,
    metadata: {},
    pages: 3,
    created_at: '2024-01-01T00:00:00Z',
    created_by: 1,
    updated_at: '2024-01-01T00:00:00Z',
    updated_by: 1,
  };
}

function fileFixture() {
  return {
    id: 1,
    document_id: 10,
    filename: 'a.pdf',
    content_type: 'application/pdf',
    size: 1024,
    pages: 2,
    scan_status: 'not_required',
    scan_requested: false,
    content_available: true,
    created_at: '2024-01-01T00:00:00Z',
    created_by: 1,
    updated_at: '2024-01-01T00:00:00Z',
    updated_by: 1,
  };
}

function renderTextContent() {
  return render(
    <MemoryRouter initialEntries={['/documents/10/text-content?file_id=1']}>
      <Routes>
        <Route path="/documents/:id/text-content" element={<ListDocumentFilePageTextContent />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('ListDocumentFilePageTextContent', () => {
  beforeEach(() => {
    mockUseDocument.mockReset();
    mockUseDocumentFiles.mockReset();
    mockUseDocumentFilePages.mockReset();
    mockUseDocument.mockReturnValue({ data: documentFixture(), isLoading: false, isError: false });
    mockUseDocumentFiles.mockReturnValue({ data: [fileFixture()], isLoading: false, isError: false });
    mockUseDocumentFilePages.mockReturnValue({
      data: [{ document_file_id: 1, page_number: 1, text_content: 'Extracted text' }],
      isLoading: false,
      isError: false,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it('explains how to check OCR when every text page is empty', () => {
    mockUseDocumentFilePages.mockReturnValue({
      data: [
        { document_file_id: 1, page_number: 1, text_content: null },
        { document_file_id: 1, page_number: 2, text_content: '   ' },
      ],
      isLoading: false,
      isError: false,
    });

    renderTextContent();

    expect(screen.getByText(/This document does not have any text/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Document OCR' })).toHaveAttribute(
      'href',
      '/documents/10/ocr-content?file_id=1',
    );
    expect(screen.queryByText('Page 1')).not.toBeInTheDocument();
  });

  it('renders extracted text without the empty-text message', () => {
    renderTextContent();

    expect(screen.getByText('Extracted text')).toBeInTheDocument();
    expect(screen.queryByText(/This document does not have any text/)).not.toBeInTheDocument();
  });
});
