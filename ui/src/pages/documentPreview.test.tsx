import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

import { DocumentFilePagePreview } from './documentFiles';

const mockUseDocument = vi.fn();
const mockUseDocumentFiles = vi.fn();
const mockUsePageImage = vi.fn();
const mockScrollToIndex = vi.fn();

vi.mock('../components/DocumentViewLayout', () => ({
  DocumentViewLayout: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock('../queries/useDocuments', () => ({
  useDocument: (id: unknown, options?: unknown) => mockUseDocument(id, options),
}));

vi.mock('../queries/useDocumentFiles', () => ({
  useDocumentFiles: (id: unknown) => mockUseDocumentFiles(id),
  useDocumentFilePageImage: (documentId: unknown, fileId: unknown, pageNumber: unknown) =>
    mockUsePageImage(documentId, fileId, pageNumber),
}));

// Render every page deterministically instead of relying on scroll measurements in jsdom.
vi.mock('@tanstack/react-virtual', () => ({
  useVirtualizer: ({ count, getItemKey }: { count: number; getItemKey?: (index: number) => string }) => ({
    getVirtualItems: () =>
      Array.from({ length: count }, (_, index) => ({
        index,
        key: getItemKey ? getItemKey(index) : `${index}`,
        start: index * 800,
        end: (index + 1) * 800,
      })),
    getTotalSize: () => count * 800,
    scrollToIndex: mockScrollToIndex,
    measure: vi.fn(),
    measureElement: () => {},
    options: { scrollMargin: 0 },
    scrollOffset: 0,
    scrollRect: { height: 800 },
  }),
}));

function documentFixture(title = 'Invoice 100') {
  return {
    id: 10,
    title,
    document_type_id: 1,
    metadata: {},
    pages: 3,
    created_at: '2024-01-01T00:00:00Z',
    created_by: 1,
    updated_at: '2024-01-01T00:00:00Z',
    updated_by: 1,
  };
}

function fileFixture(id: number, filename: string, pages = 3) {
  return {
    id,
    document_id: 10,
    filename,
    content_type: 'application/pdf',
    size: 1024,
    pages,
    created_at: '2024-01-01T00:00:00Z',
    created_by: 1,
    updated_at: '2024-01-01T00:00:00Z',
    updated_by: 1,
  };
}

function renderPreview(initialPath: string) {
  return render(
    <MemoryRouter initialEntries={[initialPath]}>
      <Routes>
        <Route path="/documents/:id/preview" element={<DocumentFilePagePreview />} />
      </Routes>
    </MemoryRouter>,
  );
}

describe('DocumentFilePagePreview', () => {
  beforeAll(() => {
    if (!('ResizeObserver' in globalThis)) {
      (globalThis as unknown as Record<string, unknown>).ResizeObserver = class {
        observe() {}
        unobserve() {}
        disconnect() {}
      };
    }
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      callback(0);
      return 0;
    });
    vi.stubGlobal('cancelAnimationFrame', () => {});
  });

  beforeEach(() => {
    mockUseDocument.mockReset();
    mockUseDocumentFiles.mockReset();
    mockUsePageImage.mockReset();
    mockScrollToIndex.mockReset();
    mockUseDocument.mockReturnValue({ data: documentFixture(), isLoading: false, isError: false });
    mockUseDocumentFiles.mockReturnValue({
      data: [fileFixture(1, 'a.pdf'), fileFixture(2, 'b.pdf')],
      isLoading: false,
      isError: false,
    });
    mockUsePageImage.mockReturnValue({ data: 'blob:image-1', isLoading: false, isError: false });
  });

  afterEach(() => {
    cleanup();
  });

  it('shows a loading state while the document loads', () => {
    mockUseDocument.mockReturnValue({ data: undefined, isLoading: true, isError: false });
    renderPreview('/documents/10/preview');

    expect(screen.getByText('Loading')).toBeInTheDocument();
  });

  it('shows a document error message', () => {
    mockUseDocument.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      error: new Error('Document not found'),
    });
    renderPreview('/documents/10/preview');

    expect(screen.getByText('Document not found')).toBeInTheDocument();
  });

  it('shows the document title and an info message when there are no files', () => {
    mockUseDocumentFiles.mockReturnValue({ data: [], isLoading: false, isError: false });
    renderPreview('/documents/10/preview');

    expect(screen.getByText('Document Preview: Invoice 100')).toBeInTheDocument();
    expect(screen.getByText('No files available for this document.')).toBeInTheDocument();
  });

  it('shows a files error message', () => {
    mockUseDocumentFiles.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      error: new Error('Files failed to load'),
    });
    renderPreview('/documents/10/preview');

    expect(screen.getByText('Files failed to load')).toBeInTheDocument();
  });

  it('shows an info message when the selected file has no pages', () => {
    mockUseDocumentFiles.mockReturnValue({ data: [fileFixture(1, 'empty.pdf', 0)], isLoading: false, isError: false });
    renderPreview('/documents/10/preview');

    expect(screen.getByText('No pages available for this file.')).toBeInTheDocument();
  });

  it('uses file_id from the URL and falls back to the first file for unknown ids', () => {
    renderPreview('/documents/10/preview?file_id=2');
    expect(screen.getByText('Page 1 of 3')).toBeInTheDocument();
    expect(mockUsePageImage).toHaveBeenCalledWith(10, 2, expect.any(Number));
    cleanup();

    mockUsePageImage.mockClear();
    renderPreview('/documents/10/preview?file_id=999');
    expect(screen.getByText('Page 1 of 3')).toBeInTheDocument();
    expect(mockUsePageImage).toHaveBeenCalledWith(10, 1, expect.any(Number));
  });

  it('renders one image per page and shows skeletons until images load', () => {
    renderPreview('/documents/10/preview?file_id=1');

    const images = screen.getAllByAltText(/Page \d+/);
    expect(images).toHaveLength(3);
    expect(images[0]).toHaveAttribute('src', 'blob:image-1');
    expect(images[0]).not.toHaveClass('is-loaded');

    fireEvent.load(images[0]);
    expect(images[0]).toHaveClass('is-loaded');
  });

  it('shows a per-page error message when the page image fails', () => {
    mockUsePageImage.mockImplementation((_docId: unknown, _fileId: unknown, pageNumber: unknown) =>
      pageNumber === 2
        ? { data: undefined, isLoading: false, isError: true, error: new Error('Page 2 failed') }
        : { data: 'blob:image-1', isLoading: false, isError: false },
    );
    renderPreview('/documents/10/preview?file_id=1');

    expect(screen.getByText('Page 2 failed')).toBeInTheDocument();
    expect(screen.getAllByAltText(/Page \d+/)).toHaveLength(2);
  });

  it('shows an info message when a page image is not available yet', () => {
    mockUsePageImage.mockReturnValue({ data: undefined, isLoading: false, isError: false });
    renderPreview('/documents/10/preview?file_id=1');

    expect(screen.getAllByText('Page image not available yet.')).toHaveLength(3);
  });

  it('navigates with previous/next buttons and updates the URL page param', async () => {
    renderPreview('/documents/10/preview?file_id=1&page=1');

    const next = screen.getByRole('button', { name: 'Next page' });
    const previous = screen.getByRole('button', { name: 'Previous page' });
    expect(previous).toBeDisabled();
    expect(next).not.toBeDisabled();

    fireEvent.click(next);
    expect(await screen.findByText('Page 2 of 3')).toBeInTheDocument();
    expect(mockScrollToIndex).toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Previous page' }));
    expect(await screen.findByText('Page 1 of 3')).toBeInTheDocument();
  });

  it('disables next on the last page', () => {
    renderPreview('/documents/10/preview?file_id=1&page=3');

    expect(screen.getByText('Page 3 of 3')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next page' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Previous page' })).not.toBeDisabled();
  });

  it('jumps to the entered page with the Go button', async () => {
    renderPreview('/documents/10/preview?file_id=1');

    const jumpInput = document.querySelector('.aut-document-preview-page-input') as HTMLInputElement;
    expect(jumpInput).not.toBeNull();
    fireEvent.input(jumpInput, { target: { value: '3' } });
    fireEvent.blur(jumpInput);
    fireEvent.click(screen.getByRole('button', { name: 'Go' }));

    expect(await screen.findByText('Page 3 of 3')).toBeInTheDocument();
  });

  it('jumps to the entered page on Enter', async () => {
    renderPreview('/documents/10/preview?file_id=1');

    const jumpInput = document.querySelector('.aut-document-preview-page-input') as HTMLInputElement;
    fireEvent.change(jumpInput, { target: { value: '2' } });
    fireEvent.keyDown(jumpInput, { key: 'Enter', code: 'Enter', currentTarget: { value: '2' } });

    expect(await screen.findByText('Page 2 of 3')).toBeInTheDocument();
  });

  it('clamps an out-of-range page param to the last page', () => {
    renderPreview('/documents/10/preview?file_id=1&page=99');

    expect(screen.getByText('Page 3 of 3')).toBeInTheDocument();
  });

  it('requests page images for the newly selected file', async () => {
    mockUseDocumentFiles.mockReturnValue({
      data: [fileFixture(7, 'first.pdf', 2), fileFixture(8, 'second.pdf', 2)],
      isLoading: false,
      isError: false,
    });
    renderPreview('/documents/10/preview?file_id=7');

    await waitFor(() => {
      expect(mockUsePageImage).toHaveBeenCalledWith(10, 7, 1);
    });
    expect(screen.getByText('Page 1 of 2')).toBeInTheDocument();
  });
});
