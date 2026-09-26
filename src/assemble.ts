import { applyTranslations, compareSkeletons, skeleton, type SkeletonItem } from './markdown/document.js';
import { preserveAnchors } from './markdown/anchors.js';
import type { Extraction, RenderOptions, TMap } from './markdown/types.js';

export interface AssembleOptions {
  wrap: boolean;
  preserveAnchors: boolean;
}

export class StructureError extends Error {}

/**
 * Splices translations into the source and proves that the Markdown structure is unchanged.
 * Blocks that would change the structure are reverted to the source text (and reported).
 */
export function assembleDocument(ex: Extraction, tm: TMap, o: AssembleOptions): { text: string; reverted: string[]; anchors: string[] } {
  const source = skeleton(ex.source, ex.parseOptions);
  const work = new Map(tm);
  const reverted: string[] = [];
  const opts: RenderOptions = { wrap: o.wrap };
  const render = () => {
    const text = applyTranslations(ex, work, opts);
    return { text, sk: skeleton(text, ex.parseOptions) };
  };
  let { text, sk } = render();
  let diff = compareSkeletons(source, sk);
  if (diff !== -1 && opts.wrap) {
    opts.wrap = false;
    ({ text, sk } = render());
    diff = compareSkeletons(source, sk);
  }
  for (let guard = 0; diff !== -1 && guard < 100; guard++) {
    const culprits = replacementsNear(ex, work, opts, sk[diff] ?? sk[sk.length - 1], sk[diff - 1]);
    if (!culprits.length) break;
    for (const id of culprits) {
      work.delete(id);
      for (const d of ex.byId.get(id)?.dependents ?? []) work.delete(d);
      reverted.push(id);
    }
    ({ text, sk } = render());
    diff = compareSkeletons(source, sk);
  }
  if (diff !== -1) throw new StructureError(`translated document failed the structure check near: ${sk[diff]?.sig.slice(0, 120) ?? 'end of document'}`);
  let anchors: string[] = [];
  if (o.preserveAnchors) {
    const r = preserveAnchors(ex.source, text, ex.parseOptions);
    text = r.text;
    anchors = r.added;
  }
  return { text: ex.bom + text, reverted, anchors };
}

/** Translated replacements overlapping the mismatching node (or, failing that, its predecessor). */
function replacementsNear(ex: Extraction, tm: TMap, opts: RenderOptions, item?: SkeletonItem, prev?: SkeletonItem): string[] {
  const ranges: { start: number; end: number; ids: string[] }[] = [];
  let pos = 0;
  let outPos = 0;
  for (const r of ex.replacements) {
    outPos += r.start - pos;
    const rendered = r.render(tm, opts);
    const ids = r.segmentIds.filter((id) => tm.has(id) || (ex.byId.get(id)?.dependents ?? []).some((d) => tm.has(d)));
    if (rendered !== ex.source.slice(r.start, r.end) && ids.length) ranges.push({ start: outPos, end: outPos + rendered.length, ids });
    outPos += rendered.length;
    pos = r.end;
  }
  const hit = (x?: SkeletonItem) => (x ? ranges.filter((r) => r.start <= x.end && r.end >= x.start).flatMap((r) => r.ids) : []);
  const a = hit(item);
  return a.length ? a : hit(prev);
}
