import { checkTags, plainText } from '../mask/masking.js';
import { parseMarkdown } from '../markdown/parse.js';
import { inlineSignature } from '../markdown/inline.js';
import { htmlSkeleton } from '../markdown/document.js';
import { renderSegment } from '../markdown/render.js';
import type { LanguageConfig } from '../config.js';
import type { Extraction, Segment } from '../markdown/types.js';

/** Normalizes model output: single line, source-like outer whitespace. */
export function sanitize(seg: Segment, text: string): string {
  const lead = /^\s*/.exec(seg.masked)![0];
  const trail = /\s*$/.exec(seg.masked)![0];
  return lead + text.replace(/\s*\r?\n\s*/g, ' ').trim() + trail;
}

export function validateSegment(seg: Segment, translated: string, ex: Extraction, lang: LanguageConfig): string[] {
  const errors = checkTags(seg, translated);
  if (errors.length) return errors;
  const text = plainText(translated);
  const srcText = plainText(seg.masked);
  for (const f of seg.forbidden ?? []) if (text.includes(f)) errors.push(`must not contain "${f}" (it would terminate the code comment)`);
  if (seg.kind === 'comment' && /\\$/.test(text.trim())) errors.push('a code comment must not end with a backslash');
  const srcLen = srcText.trim().length;
  if (srcLen >= 25) {
    const [min, max] = lang.lengthRatio ?? [0.4, 3.0];
    const ratio = text.trim().length / srcLen;
    if (ratio < min) errors.push(`translation looks incomplete (${Math.round(ratio * 100)}% of source length)`);
    if (ratio > max) errors.push(`translation looks too long (${Math.round(ratio * 100)}% of source length); do not add content`);
  }
  if (errors.length) return errors;

  const tm = new Map([[seg.id, translated]]);
  let rendered: string;
  try {
    rendered = renderSegment(seg, tm, ex.byId, { wrap: false });
  } catch (e) {
    return [`render failed: ${(e as Error).message}`];
  }
  if (seg.inlineSignature !== undefined) {
    // Headings and cells never start a line, so shield them from block-start interpretation.
    const probe = seg.kind === 'paragraph' ? rendered : `x ${rendered}`;
    const root = parseMarkdown(`${probe}\n\n${ex.definitionsText}`);
    const first = root.children[0];
    if (!first || first.type !== 'paragraph') return ['translation turns into a different Markdown block (e.g. list or heading); rephrase the start'];
    const sig = inlineSignature(first.children, seg.kind === 'cell');
    if (sig !== seg.inlineSignature) {
      return [
        'inline formatting changed after rendering; keep emphasis/link tags hugging whole words (no spaces directly inside tags, no letters directly attached outside underscore-emphasis tags) and do not introduce Markdown characters',
      ];
    }
  }
  if (seg.kind === 'html' && htmlSkeleton(rendered) !== htmlSkeleton(seg.original)) errors.push('HTML structure changed');
  return errors;
}

/** Soft check used for reporting: long segments that came back unchanged. */
export function looksUntranslated(seg: Segment, translated: string): boolean {
  const src = plainText(seg.masked).trim();
  return src.split(/\s+/).filter((w) => /\p{L}{3,}/u.test(w)).length >= 4 && plainText(translated).trim() === src;
}
