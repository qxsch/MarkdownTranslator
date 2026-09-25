import { joinPieces, wrapPieces } from './wrap.js';
import { renderPieces } from '../mask/masking.js';
import type { RenderOptions, Segment, TMap } from './types.js';

const PARAGRAPH_BLOCK_START = /^(?:[-+*](?=[ \t]|$)|#{1,6}(?=[ \t]|$)|>|\d{1,9}(?=[.)](?:[ \t]|$))|={2,}[ \t]*$|`{3}|~{3})/;

function isTranslated(seg: Segment, tm: TMap): boolean {
  const t = tm.get(seg.id);
  return t !== undefined && t !== seg.masked;
}

export function needsRender(seg: Segment, tm: TMap, all: ReadonlyMap<string, Segment>): boolean {
  if (isTranslated(seg, tm)) return true;
  return (seg.dependents ?? []).some((id) => {
    const d = all.get(id);
    return d ? needsRender(d, tm, all) : false;
  });
}

export function renderSegment(seg: Segment, tm: TMap, all: ReadonlyMap<string, Segment>, opts: RenderOptions = {}): string {
  if (!opts.force && !needsRender(seg, tm, all)) return seg.original;
  const translated = tm.get(seg.id) ?? seg.masked;
  const pieces = renderPieces(seg, translated, tm, seg.sourceText);
  let out = seg.wrap && opts.wrap !== false ? wrapPieces(pieces, seg.wrap) : joinPieces(pieces);
  if (seg.kind === 'paragraph') out = escapeParagraphStart(out, seg.original);
  if (seg.finalize) out = seg.finalize(out);
  return out;
}

/** A translated paragraph must not start with characters that would turn it into another block type. */
export function escapeParagraphStart(out: string, original: string): string {
  const m = PARAGRAPH_BLOCK_START.exec(out);
  if (!m || PARAGRAPH_BLOCK_START.exec(original)?.[0] === m[0]) return out;
  if (/^\d/.test(m[0])) return out.slice(0, m[0].length) + '\\' + out.slice(m[0].length);
  return '\\' + out;
}
