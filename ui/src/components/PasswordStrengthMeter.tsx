import { classNames } from "primereact/utils";

import {
  getPasswordStrength,
  passwordStrengthLabel,
} from "./passwordStrength";

// Inline strength meter rendered above the password input.
//
// The PrimeReact Password overlay panel always drops below the input
// (it only flips when the viewport forces it), which puts it under
// password-manager suggestion popups. This inline meter is always
// visible and cannot be covered by such popups.
export function PasswordStrengthMeter({ value }: Readonly<{ value: string | null | undefined }>) {
  const strength = getPasswordStrength(value);

  return (
    <div className="aut-password-meter" aria-live="polite">
      <div className="aut-password-meter-track" aria-hidden="true">
        <div className={classNames("aut-password-meter-fill", `is-${strength}`)} />
      </div>
      <small className={classNames("aut-password-meter-label", `is-${strength}`)}>
        Strength: {passwordStrengthLabel(strength)}
      </small>
    </div>
  );
}
