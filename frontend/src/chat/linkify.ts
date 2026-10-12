export interface LinkSegment {
  text: string;
  href: string | null;
}

const URL_RE = /(https?:\/\/[^\s<>"']+)/gi;
const HAS_URL_RE = /https?:\/\/[^\s<>"']+/i;

/**
 * Split text into plain segments and clickable URLs. URLs are emitted as
 * `href` segments; everything else is plain text.
 */
export function linkified(text: string): LinkSegment[] {
  const segments: LinkSegment[] = [];
  let cursor = 0;
  for (const match of text.matchAll(URL_RE)) {
    const index = match.index ?? 0;
    if (index > cursor) {
      segments.push({ text: text.slice(cursor, index), href: null });
    }
    const raw = match[0];
    // Sentence punctuation right after a URL belongs to the text, not the link.
    const trailing = raw.match(/[.,;:!?)\]}'"]+$/)?.[0] ?? "";
    const url = raw.slice(0, raw.length - trailing.length);
    if (url.length > 0) {
      segments.push({ text: url, href: url });
    }
    if (trailing.length > 0) {
      segments.push({ text: trailing, href: null });
    }
    cursor = index + raw.length;
  }
  if (cursor < text.length) {
    segments.push({ text: text.slice(cursor), href: null });
  }
  return segments;
}

export function hasAnyLink(text: string): boolean {
  return HAS_URL_RE.test(text);
}
