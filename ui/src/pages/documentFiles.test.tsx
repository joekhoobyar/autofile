import { cleanup, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ListDocumentFiles } from './documentFiles';

const mockUseDocument = vi.fn();
const mockUseDocumentFiles = vi.fn();
const mockUsePublicSettings = vi.fn();
const mockUseThumbnail = vi.fn();

vi.mock('../components/DocumentViewLayout', () => ({
  DocumentViewLayout: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock('../queries/useDocuments', () => ({
  useDocument: (id: unknown, options?: unknown) => mockUseDocument(id, options),
}));

vi.mock('../queries/useDocumentFiles', () => ({
  useDocumentFiles: (id: unknown) => mockUseDocumentFiles(id),
  useDocumentFileThumbnail: (documentId: unknown, fileId: unknown, options?: unknown) =>
    mockUseThumbnail(documentId, fileId, options),
  useDeleteDocumentFile: () => ({ mutateAsync: vi.fn() }),
  useRescanDocumentFile: () => ({ mutateAsync: vi.fn() }),
}));

vi.mock('../queries/useAppSettings', () => ({
  usePublicSettings: () => mockUsePublicSettings(),
}));

function fileFixture(overrides: Record<string, unknown> = {}) {
  return {
    id: 1,
    document_id: 10,
    filename: 'a.pdf',
    content_type: 'application/pdf',
    size: 1024,
    pages: 2,
    scan_status: 'pending',
    scan_requested: true,
    content_available: false,
    created_at: '2024-01-01T00:00:00Z',
    created_by: 1,
    updated_at: '2024-01-01T00:00:00Z',
    updated_by: 1,
    ...overrides,
  };
}

function renderList() {
  return render(
    <MemoryRouter initialEntries={['/documents/10/files']}>
      <Routes>
        <Route path="/documents/:id/files" element={<ListDocumentFiles />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('ListDocumentFiles virus scan gating', () => {
  beforeEach(() => {
    mockUseDocument.mockReset();
    mockUseDocumentFiles.mockReset();
    mockUsePublicSettings.mockReset();
    mockUseThumbnail.mockReset();
    mockUseDocument.mockReturnValue({ data: { id: 10, title: 'Invoice' }, isLoading: false, isError: false });
    mockUsePublicSettings.mockReturnValue({
      data: { virus_scanning_enabled: true, virus_scan_by_default: true },
    });
    mockUseThumbnail.mockReturnValue({ data: undefined });
  });

  afterEach(() => {
    cleanup();
  });

  it('disables Download when content_available is false', () => {
    mockUseDocumentFiles.mockReturnValue({
      data: [fileFixture({ content_available: false, scan_status: 'pending' })],
      isLoading: false,
      isError: false,
    });
    renderList();

    const download = screen.getByRole('button', { name: 'Download a.pdf' });
    expect(download).toBeDisabled();
    expect(
      screen.getByText(
        'This file is stored but is waiting for virus scanning. It will be available after a clean scan.',
      ),
    ).toBeInTheDocument();
  });

  it('enables Download when content_available is true', () => {
    mockUseDocumentFiles.mockReturnValue({
      data: [fileFixture({ content_available: true, scan_status: 'clean' })],
      isLoading: false,
      isError: false,
    });
    renderList();

    const download = screen.getByRole('button', { name: 'Download a.pdf' });
    expect(download).not.toBeDisabled();
  });
});
