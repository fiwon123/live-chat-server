// Deterministic avatar colors/initials (pure helpers).

const AVATAR_COLORS = [
  "#14b8a6",
  "#0ea5e9",
  "#6366f1",
  "#a855f7",
  "#ec4899",
  "#f97316",
  "#84cc16",
  "#10b981",
];

/** Deterministic color from a name, stable across renders and clients. */
export function avatarColor(name: string): string {
  let hash = 0;
  for (const char of name) {
    hash = (hash * 31 + char.codePointAt(0)!) >>> 0;
  }
  return AVATAR_COLORS[hash % AVATAR_COLORS.length];
}

/** First letters of the first two words, uppercased ("alice" → "A"). */
export function initialsOf(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  return (parts.length > 0 ? parts.slice(0, 2).map((part) => part[0]!) : ["?"])
    .join("")
    .toUpperCase();
}
