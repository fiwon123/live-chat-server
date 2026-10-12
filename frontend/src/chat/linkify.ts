export interface LinkSegment {
  text: string;
  href: string | null;
}

const URL_RE = /(https?:\/\/[^\s<>"']+)/gi;

const OPEN_FOR: Record<string, string> = { ")": "(", "]": "[", "}": "{" };

function countChar(text: string, char: string): number {
  let count = 0;
  for (const candidate of text) {
    if (candidate === char) {
      count += 1;
    }
  }
  return count;
}

/** Split a matched URL into the link and any dangling sentence punctuation. */
function splitTrailingPunctuation(raw: string): [string, string] {
  let url = raw;
  let trailing = "";

  // Sentence punctuation is never part of a URL.
  while (url.length > 0 && /[.,;:!?'"]/.test(url[url.length - 1]!)) {
    trailing = url[url.length - 1] + trailing;
    url = url.slice(0, -1);
  }

  // A closing bracket is stripped only when it is unbalanced, so
  // ".../Foo_(bar)" keeps its parentheses while "(see https://x.com)" does not.
  let changed = true;
  while (changed && url.length > 0) {
    changed = false;
    const last = url[url.length - 1]!;
    const opener = OPEN_FOR[last];
    if (opener !== undefined && countChar(url, last) > countChar(url, opener)) {
      trailing = last + trailing;
      url = url.slice(0, -1);
      changed = true;
    }
  }

  return [url, trailing];
}

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
    const [url, trailing] = splitTrailingPunctuation(match[0]);
    if (url.length > 0) {
      segments.push({ text: url, href: url });
    }
    if (trailing.length > 0) {
      segments.push({ text: trailing, href: null });
    }
    cursor = index + match[0].length;
  }
  if (cursor < text.length) {
    segments.push({ text: text.slice(cursor), href: null });
  }
  return segments;
}
