import { useId } from "react";

interface LogoProps {
  /** Pixel size of the mark. */
  size?: number;
  /** Render the "Live Chat" wordmark next to the mark. */
  withWordmark?: boolean;
  className?: string;
}

/** The Live Chat mark: a speech bubble carrying a pulse. */
export function LogoMark({
  size = 26,
  decorative = false,
}: {
  size?: number;
  decorative?: boolean;
}) {
  const gradientId = useId();
  const a11y = decorative
    ? ({ "aria-hidden": true } as const)
    : ({ role: "img", "aria-label": "Live Chat" } as const);

  return (
    <svg width={size} height={size} viewBox="0 0 32 32" focusable="false" {...a11y}>
      <defs>
        <linearGradient
          id={gradientId}
          x1="4"
          y1="5"
          x2="28"
          y2="25"
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0" stopColor="#5eead4" />
          <stop offset="1" stopColor="#14b8a6" />
        </linearGradient>
      </defs>
      <path
        d="M26 5H6a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h2v4l5-4h13a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2z"
        fill={`url(#${gradientId})`}
      />
      <path
        d="M10.5 13h3l1.6-3.2 2.2 6 1.5-2.8h3.7"
        fill="none"
        stroke="#062e2a"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function Logo({ size = 26, withWordmark = true, className }: LogoProps) {
  return (
    <span className={className ? `logo ${className}` : "logo"}>
      <LogoMark size={size} decorative={withWordmark} />
      {withWordmark ? <span className="logo__word">Live Chat</span> : null}
    </span>
  );
}
