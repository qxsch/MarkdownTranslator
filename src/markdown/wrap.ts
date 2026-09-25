import type { Piece } from '../mask/masking.js';
import type { WrapSpec } from './types.js';

interface Word {
  text: string;
  spaceBefore: boolean;
}

/** A word at the start of a line must not turn into block syntax (list item, heading, quote, fence, HTML block...). */
const UNSAFE_LINE_START = [
  /^(?:[-+*]|\d{1,9}[.)]|#{1,6})$/,
  /^(?:>|`{3,}|~{3,}|<[A-Za-z!/?]|\|)/,
  /^(?:=+|-+|_+|\*+)$/,
];

export function safeLineStart(word: string): boolean {
  return !UNSAFE_LINE_START.some((re) => re.test(word));
}

function toWords(pieces: Piece[]): Word[] {
  const words: Word[] = [];
  let cur = '';
  let spaceBefore = false;
  let pendingSpace = false;
  const flush = () => {
    if (cur) words.push({ text: cur, spaceBefore });
    cur = '';
  };
  for (const p of pieces) {
    if (p.atom) {
      if (pendingSpace) {
        flush();
        spaceBefore = true;
        pendingSpace = false;
      }
      cur += p.s;
      continue;
    }
    for (const part of p.s.split(/( +)/)) {
      if (!part) continue;
      if (part.startsWith(' ')) {
        if (cur) {
          flush();
          spaceBefore = false;
        }
        pendingSpace = true;
      } else {
        if (pendingSpace) {
          flush();
          spaceBefore = true;
          pendingSpace = false;
        }
        cur += part;
      }
    }
  }
  flush();
  if (pendingSpace) words.push({ text: '', spaceBefore: true });
  return words;
}

export function joinPieces(pieces: Piece[]): string {
  return pieces.map((p) => p.s).join('');
}

/** Greedy re-wrap to the source width; placeholders (inline code, link targets, breaks) are never split. */
export function wrapPieces(pieces: Piece[], spec: WrapSpec): string {
  const words = toWords(pieces);
  let out = '';
  let col = spec.firstColumn;
  let lineHasContent = false;
  for (const w of words) {
    const sp = w.spaceBefore ? ' ' : '';
    const nl = w.text.indexOf('\n');
    const firstLen = nl === -1 ? w.text.length : nl;
    if (lineHasContent && sp && w.text && col + 1 + firstLen > spec.width && safeLineStart(w.text)) {
      out += spec.eol + spec.prefix + w.text;
      col = spec.prefix.length;
    } else {
      out += sp + w.text;
      col += sp.length;
    }
    if (nl === -1) {
      col += w.text.length;
      lineHasContent = lineHasContent || w.text.length > 0;
    } else {
      const tail = w.text.slice(w.text.lastIndexOf('\n') + 1);
      col = tail.length;
      // After a hard break the next word continues right after the break's prefix.
      lineHasContent = false;
    }
  }
  return out;
}
