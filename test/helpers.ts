import { tokenize, xmlEscape, xmlUnescape } from '../src/mask/masking.js';
import type { Extraction } from '../src/markdown/types.js';
import type { LanguageConfig } from '../src/config.js';

export const LANG: LanguageConfig = { code: 'de', name: 'German', formal: '', informal: '' };

/** Pseudo translation: every word gets a prefix and is upper-cased; tags stay where they are. */
export const pseudoWords = (s: string) => s.replace(/\p{L}+/gu, (w) => `Ü${w.toUpperCase()}`);

export function pseudo(masked: string): string {
  return tokenize(masked)
    .map((t) => (t.t === 'text' ? xmlEscape(pseudoWords(xmlUnescape(t.v))) : t.t === 'x' ? `<x${t.n}/>` : t.t === 'open' ? `<g${t.n}>` : `</g${t.n}>`))
    .join('');
}

export function pseudoMap(ex: Extraction) {
  return new Map(ex.segments.filter((s) => !s.passive).map((s) => [s.id, pseudo(s.masked)]));
}
