import type { ReactNode, SVGProps } from "react";

export type IconName =
  | "hash"
  | "plus"
  | "send"
  | "smile"
  | "user"
  | "logout"
  | "close"
  | "search"
  | "eye"
  | "eye-off"
  | "check"
  | "alert"
  | "spinner"
  | "chevron-down"
  | "chevron-left"
  | "moon"
  | "sun"
  | "bell"
  | "arrow-down"
  | "message"
  | "menu"
  | "more";

const PATHS: Record<IconName, ReactNode> = {
  hash: (
    <>
      <path d="M4 9h16M4 15h16M10 3 8 21M16 3l-2 18" />
    </>
  ),
  plus: <path d="M12 5v14M5 12h14" />,
  send: <path d="M22 2 11 13M22 2l-7 20-4-9-9-4 20-7z" />,
  smile: (
    <>
      <path d="M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z" />
      <path d="M8.5 14.5s1.3 2 3.5 2 3.5-2 3.5-2" />
      <path d="M9 9.5h.01M15 9.5h.01" />
    </>
  ),
  user: (
    <>
      <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
      <path d="M12 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8z" />
    </>
  ),
  logout: (
    <>
      <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
      <path d="M16 17l5-5-5-5M21 12H9" />
    </>
  ),
  close: <path d="M18 6 6 18M6 6l12 12" />,
  search: (
    <>
      <path d="M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16z" />
      <path d="M21 21l-4.35-4.35" />
    </>
  ),
  eye: (
    <>
      <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7z" />
      <path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
    </>
  ),
  "eye-off": (
    <>
      <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-10-8-10-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 10 8 10 8a18.5 18.5 0 0 1-2.16 3.19" />
      <path d="M14.12 14.12a3 3 0 1 1-4.24-4.24" />
      <path d="M1 1l22 22" />
    </>
  ),
  check: <path d="M20 6 9 17l-5-5" />,
  alert: (
    <>
      <path d="M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z" />
      <path d="M12 8v4M12 16h.01" />
    </>
  ),
  spinner: <path d="M21 12a9 9 0 1 1-6.2-8.56" />,
  "chevron-down": <path d="M6 9l6 6 6-6" />,
  "chevron-left": <path d="M15 18l-6-6 6-6" />,
  moon: <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />,
  sun: (
    <>
      <path d="M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10z" />
      <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
    </>
  ),
  bell: (
    <>
      <path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9" />
      <path d="M13.73 21a2 2 0 0 1-3.46 0" />
    </>
  ),
  "arrow-down": <path d="M12 5v14M19 12l-7 7-7-7" />,
  message: (
    <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" />
  ),
  menu: <path d="M3 12h18M3 6h18M3 18h18" />,
  more: <path d="M5 12h.01M12 12h.01M19 12h.01" />,
};

export interface IconProps extends SVGProps<SVGSVGElement> {
  name: IconName;
  size?: number;
  strokeWidth?: number;
}

/** Self-contained inline SVG icon (24x24 grid, currentColor stroke). */
export function Icon({ name, size = 20, strokeWidth = 1.8, ...rest }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {PATHS[name]}
    </svg>
  );
}
