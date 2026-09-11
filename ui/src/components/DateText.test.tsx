import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { DateText } from './DateText';

describe('DateText', () => {
  it('renders a formatted date value', () => {
    render(<DateText value={new Date(2024, 0, 2, 3, 4)} />);

    expect(screen.getByText('01/02/2024 03:04')).toBeInTheDocument();
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
