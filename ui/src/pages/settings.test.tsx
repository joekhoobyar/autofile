import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { Settings } from './settings';

const mockUseAppSettings = vi.fn();
const mockUseSaveAppSettings = vi.fn();

vi.mock('../queries/useAppSettings', () => ({
  useAppSettings: () => mockUseAppSettings(),
  useSaveAppSettings: () => mockUseSaveAppSettings(),
}));

function settingsFixture(virusScanningEnabled: boolean) {
  return {
    id: 1,
    allow_user_registration: true,
    date_format: 'yyyy-MM-dd',
    datetime_format: 'MM/dd/yyyy HH:mm',
    virus_scanning_enabled: virusScanningEnabled,
    virus_scan_by_default: true,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  };
}

describe('Settings virus scanning controls', () => {
  beforeEach(() => {
    mockUseSaveAppSettings.mockReturnValue({
      mutateAsync: vi.fn(),
      isError: false,
      isPending: false,
    });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('hides scan-by-default when virus scanning is disabled', () => {
    mockUseAppSettings.mockReturnValue({
      data: settingsFixture(false),
      isPending: false,
      isError: false,
    });

    render(<Settings />);

    expect(screen.getByLabelText('Enable virus scanning')).toBeInTheDocument();
    expect(screen.queryByLabelText('Virus scan uploads by default')).not.toBeInTheDocument();
  });

  it('shows scan-by-default when virus scanning is enabled', () => {
    mockUseAppSettings.mockReturnValue({
      data: settingsFixture(true),
      isPending: false,
      isError: false,
    });

    render(<Settings />);

    expect(screen.getByLabelText('Enable virus scanning')).toBeChecked();
    expect(screen.getByLabelText('Virus scan uploads by default')).toBeChecked();
  });
});
