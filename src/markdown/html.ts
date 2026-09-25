import { parseFragment } from 'parse5';
import type { DefaultTreeAdapterMap } from 'parse5';
import type { Html } from 'mdast';
import type { ExtractContext } from './context.js';
import type { MaskBuilder } from '../mask/masking.js';
import { INLINE_ELEMENTS, NO_TRANSLATE_ELEMENTS, TRANSLATABLE_ATTRS, VOID_ELEMENTS, hasNoTranslateMarker } from './htmlAttrs.js';
import { buildLineMap, lineStartOf, maxLineWidth } from './lineMap.js';
import { endOf, startOf } from './parse.js';

type PNode = DefaultTreeAdapterMap['childNode'];
type PElement = DefaultTreeAdapterMap['element'];

interface Loc {
  startOffset: number;
  endOffset: number;
}

const isElement = (n: PNode): n is PElement => 'tagName' in n;
const loc = (n: PNode): Loc | undefined => (n as { sourceCodeLocation?: Loc | null }).sourceCodeLocation ?? undefined;

function isNoTranslate(el: PElement, startTag: string): boolean {
  return NO_TRANSLATE_ELEMENTS.has(el.tagName) || hasNoTranslateMarker(startTag);
}

function isInlineNode(n: PNode): boolean {
  if (n.nodeName === '#text' || n.nodeName === '#comment') return true;
  if (!isElement(n)) return false;
  if (!INLINE_ELEMENTS.has(n.tagName)) return false;
  return n.childNodes.every(isInlineNode);
}

export function processHtmlBlock(ctx: ExtractContext, node: Html) {
  const src = ctx.source;
  const s = startOf(node);
  const e = endOf(node);
  const value = node.value;
  const map = value === src.slice(s, e) ? (v: number) => v + s : buildLineMap(src, lineStartOf(src, s), value);
  if (!map) {
    ctx.notes.push(`HTML block at offset ${s} left untranslated (could not map container prefixes)`);
    return;
  }
  const slice = (vs: number, ve: number) => src.slice(map(vs), map(ve));
  const frag = parseFragment(value, { sourceCodeLocationInfo: true });

  const startTagOf = (el: PElement): Loc | undefined => el.sourceCodeLocation?.startTag ?? undefined;

  const blockAttributes = (el: PElement) => {
    const st = startTagOf(el);
    if (!st || !el.attrs.some((a) => TRANSLATABLE_ATTRS.has(a.name))) return;
    const tag = slice(st.startOffset, st.endOffset);
    const t = ctx.tagWithAttributes(tag, `HTML <${el.tagName}>`);
    if (!t.ids.length) return;
    const mb = ctx.builder();
    mb.placeholder(t.render, tag);
    const seg = ctx.segment(mb, { kind: 'html', textContext: 'html', original: tag, note: 'HTML tag', dependents: t.ids, force: true });
    if (seg) ctx.replace(map(st.startOffset), map(st.endOffset), seg);
  };

  const emit = (n: PNode, mb: MaskBuilder, deps: string[], clampStart: number, clampEnd: number) => {
    const l = loc(n);
    if (!l) throw new Error('node without location');
    const vs = Math.max(l.startOffset, clampStart);
    const ve = Math.min(l.endOffset, clampEnd);
    if (n.nodeName === '#text') {
      if (ve > vs) mb.source(slice(vs, ve), 'html');
      return;
    }
    if (!isElement(n)) {
      mb.placeholder(slice(l.startOffset, l.endOffset), slice(l.startOffset, l.endOffset));
      return;
    }
    const st = startTagOf(n);
    const et = n.sourceCodeLocation?.endTag ?? undefined;
    if (!st) throw new Error('element without start tag location');
    const tag = slice(st.startOffset, st.endOffset);
    if (VOID_ELEMENTS.has(n.tagName) || !et || isNoTranslate(n, tag)) {
      if (VOID_ELEMENTS.has(n.tagName) && !et) {
        const t = ctx.tagWithAttributes(tag, `inline HTML <${n.tagName}>`);
        deps.push(...t.ids);
        mb.placeholder(t.render, tag);
      } else {
        mb.placeholder(slice(l.startOffset, l.endOffset), slice(l.startOffset, l.endOffset));
      }
      return;
    }
    const t = ctx.tagWithAttributes(tag, `inline HTML <${n.tagName}>`);
    deps.push(...t.ids);
    const pair = mb.open('html', t.render, slice(et.startOffset, et.endOffset), `<${n.tagName}>`);
    for (const c of n.childNodes) emit(c, mb, deps, clampStart, clampEnd);
    mb.close(pair);
  };

  const flushRun = (run: PNode[]) => {
    if (!run.length) return;
    const locs = run.map(loc);
    if (locs.some((x) => !x) || locs.some((x, i) => i > 0 && x!.startOffset < locs[i - 1]!.endOffset)) return;
    let vs = locs[0]!.startOffset;
    let ve = locs[locs.length - 1]!.endOffset;
    const first = run[0];
    const last = run[run.length - 1];
    if (first.nodeName === '#text') vs += /^\s*/.exec(value.slice(vs, ve))![0].length;
    if (last.nodeName === '#text') ve -= /\s*$/.exec(value.slice(vs, ve))![0].length;
    if (ve <= vs) return;
    const mb = ctx.builder();
    const deps: string[] = [];
    for (const n of run) emit(n, mb, deps, vs, ve);
    const from = map(vs);
    const to = map(ve);
    const wrap = mb.softBreak
      ? { width: Math.max(maxLineWidth(src, from, to), 40), firstColumn: from - lineStartOf(src, from), prefix: mb.softBreak.prefix, eol: mb.softBreak.eol }
      : undefined;
    const seg = ctx.segment(mb, { kind: 'html', textContext: 'html', original: src.slice(from, to), note: 'HTML text', wrap, dependents: deps, force: true });
    if (seg) ctx.replace(from, to, seg);
  };

  const walk = (nodes: PNode[]) => {
    let run: PNode[] = [];
    for (const n of nodes) {
      if (isInlineNode(n)) {
        run.push(n);
        continue;
      }
      flushRun(run);
      run = [];
      if (!isElement(n)) continue;
      const st = startTagOf(n);
      const tag = st ? slice(st.startOffset, st.endOffset) : '';
      if (st && isNoTranslate(n, tag)) continue;
      if (st) blockAttributes(n);
      const children = n.tagName === 'template' ? (n as DefaultTreeAdapterMap['template']).content.childNodes : n.childNodes;
      walk(children);
    }
    flushRun(run);
  };

  walk(frag.childNodes);
}
