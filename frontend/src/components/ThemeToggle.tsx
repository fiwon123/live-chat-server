import type { Theme } from "../hooks/useTheme";
import { Icon } from "./Icon";

interface ThemeToggleProps {
  theme: Theme;
  onToggle: () => void;
  /**
   * Look of the button. Defaults to the topbar's ghost icon button; the auth
   * screen passes its pill class so the control matches the About link there.
   */
  className?: string;
}

/**
 * One dark/light toggle button. The icon and accessible name are derived from
 * the current theme, so callers only choose the surface styling (`className`).
 *
 * The theme itself lives in `App` (via `useTheme`) — callers pass the value and
 * the toggle handler down rather than calling `useTheme` again, which would
 * create a second instance fighting over `data-theme` and localStorage.
 */
export function ThemeToggle({
  theme,
  onToggle,
  className = "btn btn--ghost btn--icon",
}: ThemeToggleProps) {
  const next = theme === "dark" ? "light" : "dark";
  return (
    <button
      type="button"
      className={className}
      onClick={onToggle}
      aria-label={`Switch to ${next} theme`}
      title="Toggle theme"
    >
      <Icon name={theme === "dark" ? "sun" : "moon"} size={18} />
    </button>
  );
}
