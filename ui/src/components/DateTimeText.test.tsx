import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { DateTimeText } from './DateTimeText';

vi.mock('../queries/useAppSettings', () => ({
  usePublicSettings: () => ({ data: { datetime_format: 'yyyy-MM-dd HH:mm' } }),
}));

describe('DateTimeText', () => {
  it('renders a formatted date and time value', () => {
    render(<DateTimeText value={new Date(2024, 0, 2, 3, 4)} />);

    expect(screen.getByText('2024-01-02 03:04')).toBeInTheDocument();
  });

  it('renders the fallback for missing values', () => {
    render(<DateTimeText value={null} fallback="No date" />);

    expect(screen.getByText('No date')).toBeInTheDocument();
  });

  it('renders the fallback for invalid date strings', () => {
    render(<DateTimeText value="not a date" fallback="Invalid date" />);

    expect(screen.getByText('Invalid date')).toBeInTheDocument();
  });
});
