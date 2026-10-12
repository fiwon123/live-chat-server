interface LogoProps {
  /** Pixel size of the mark. */
  size?: number;
  /** Render the "live-chat" wordmark next to the mark. */
  withWordmark?: boolean;
  className?: string;
}

/** The live-chat mark: a speech bubble carrying a pulse. */
export function LogoMark({ size = 26 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      role="img"
      aria-label="live-chat"
      focusable="false"
    >
      <defs>
        <linearGradient id="lcMark" x1="4" y1="5" x2="28" y2="25" gradientUnits="userSpaceOnUse">
          <stop offset="0" stopColor="#5eead4" />
          <stop offset="1" stopColor="#14b8a6" />
        </linearGradient>
      </defs>
      <path
        d="M26 5H6a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h2v4l5-4h13a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2z"
        fill="url(#lcMark)"
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
      <LogoMark size={size} />
      {withWordmark ? <span className="logo__word">live-chat</span> : null}
    </span>
  );
}
