import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { DateText } from './DateText';

vi.mock('../queries/useAppSettings', () => ({
  usePublicSettings: () => ({ data: { date_format: 'dd/MM/yyyy' } }),
}));

describe('DateText', () => {
  it('renders a formatted date value', () => {
    render(<DateText value="2024-01-02" />);

    expect(screen.getByText('02/01/2024')).toBeInTheDocument();
  });

  it('allows explicit format overrides', () => {
    render(<DateText value="2024-01-02" formatString="yyyy-MM-dd" />);

    expect(screen.getByText('2024-01-02')).toBeInTheDocument();
  });

  it('renders the fallback for missing values', () => {
    render(<DateText value={null} fallback="No date" />);

    expect(screen.getByText('No date')).toBeInTheDocument();
  });

  it('renders the fallback for invalid date strings', () => {
    render(<DateText value="not a date" fallback="Invalid date" />);

    expect(screen.getByText('Invalid date')).toBeInTheDocument();
  });
});
