export const USERNAME_MIN = 3;
export const USERNAME_MAX = 32;
export const PASSWORD_MIN = 8;
export const PASSWORD_MAX = 128;

/** Backend rule: 3–32 chars of a–z, 0–9 and underscore. */
export function isValidUsername(value: string): boolean {
  return new RegExp(`^[a-z0-9_]{${USERNAME_MIN},${USERNAME_MAX}}$`).test(value);
}

/** Backend rule: 8–128 chars. */
export function isValidPassword(value: string): boolean {
  return value.length >= PASSWORD_MIN && value.length <= PASSWORD_MAX;
}

export interface CredentialRule {
  id: string;
  label: string;
  met: boolean;
}

/** Live checklist shown while registering. */
export function credentialRules(username: string, password: string): CredentialRule[] {
  return [
    {
      id: "username",
      label: `${USERNAME_MIN}–${USERNAME_MAX} characters (a–z, 0–9, _)`,
      met: isValidUsername(username),
    },
    {
      id: "password",
      label: `At least ${PASSWORD_MIN} characters`,
      met: password.length >= PASSWORD_MIN,
    },
  ];
}

export function credentialsValid(username: string, password: string): boolean {
  return isValidUsername(username) && isValidPassword(password);
}
