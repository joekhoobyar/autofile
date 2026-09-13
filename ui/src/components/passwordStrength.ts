// Shared PrimeReact Password strength-meter configuration.
//
// Advisory only: the backend enforces a 12-character minimum
// (see api/src/shared/auth.rs). These regexes only drive the
// weak/medium/strong display so the meter never calls a password
// "medium" or "strong" when the API would still reject it for length.
//
// - Weak: anything under 12 characters (or without enough variety).
// - Medium: 12+ characters with at least 2 of: lowercase, uppercase, digit.
// - Strong: 12+ characters with lowercase + uppercase + digit + symbol.
export const passwordPromptLabel = "Enter a password";
export const passwordWeakLabel = "Weak";
export const passwordMediumLabel = "Medium";
export const passwordStrongLabel = "Strong";

export const passwordMediumRegex =
  "^(((?=.*[a-z])(?=.*[A-Z]))|((?=.*[a-z])(?=.*[0-9]))|((?=.*[A-Z])(?=.*[0-9])))(?=.{12,})";

export const passwordStrongRegex =
  "^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9])(?=.*[^A-Za-z0-9])(?=.{12,})";

export type PasswordStrength = "empty" | "weak" | "medium" | "strong";

const mediumPattern = new RegExp(passwordMediumRegex);
const strongPattern = new RegExp(passwordStrongRegex);

export function getPasswordStrength(value: string | null | undefined): PasswordStrength {
  if (!value) {
    return "empty";
  }
  if (strongPattern.test(value)) {
    return "strong";
  }
  if (mediumPattern.test(value)) {
    return "medium";
  }
  return "weak";
}

export function passwordStrengthLabel(strength: PasswordStrength): string {
  switch (strength) {
    case "strong":
      return passwordStrongLabel;
    case "medium":
      return passwordMediumLabel;
    case "weak":
      return passwordWeakLabel;
    case "empty":
      return passwordPromptLabel;
  }
}
