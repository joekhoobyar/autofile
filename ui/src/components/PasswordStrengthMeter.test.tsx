import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { PasswordStrengthMeter } from './PasswordStrengthMeter';

describe('PasswordStrengthMeter', () => {
  it('shows the prompt when empty', () => {
    render(<PasswordStrengthMeter value="" />);

    expect(screen.getByText('Strength: Enter a password')).toBeInTheDocument();
  });

  it('shows weak for a short password', () => {
    render(<PasswordStrengthMeter value="short" />);

    expect(screen.getByText('Strength: Weak')).toBeInTheDocument();
  });

  it('shows strong for a complex 12+ character password', () => {
    render(<PasswordStrengthMeter value="Abc123456789!" />);

    expect(screen.getByText('Strength: Strong')).toBeInTheDocument();
  });
});
