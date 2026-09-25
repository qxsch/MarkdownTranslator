import type { Nodes, PhrasingContent } from 'mdast';
import type { MaskBuilder } from '../mask/masking.js';
import type { ExtractContext } from './context.js';
import { endOf, startOf } from './parse.js';
import { NO_TRANSLATE_ELEMENTS, VOID_ELEMENTS, hasNoTranslateMarker, isClosingTag, isSelfClosing, tagName, tagSkeleton } from './htmlAttrs.js';
import type { TMap } from './types.js';

/** Index of the `]` matching the `[` at `open`, honoring escapes and code spans; -1 if none. */
export function findLabelEnd(src: string, open: number, limit: number): number {
  let depth = 0;
  for (let i = open; i < limit; i++) {
    const c = src[i];
    if (c === '\\') {
      i++;
      continue;
    }
    if (c === '`') {
      const run = /^`+/.exec(src.slice(i, limit))![0];
      const close = src.indexOf(run, i + run.length);
      i = close !== -1 && close < limit ? close + run.length - 1 : i + run.length - 1;
      continue;
    }
    if (c === '[') depth++;
    else if (c === ']' && --depth === 0) return i;
  }
  return -1;
}

const DESTINATION = /^\]\(\s*(?:<(?:\\.|[^\\<>\n])*>|(?:\\.|[^\s\\()]|\((?:\\.|[^\s\\()])*\))*)\s+/;

/** Splits `](url "title")` into the part before the title text, the raw title and the rest. */
export function splitTitle(close: string): { before: string; raw: string; after: string; quote: string } | null {
  const end = /(["')])(\s*\))$/.exec(close);
  const dest = DESTINATION.exec(close);
  if (!end || !dest) return null;
  const opening = end[1] === ')' ? '(' : end[1];
  const start = dest[0].length;
  if (close[start] !== opening || start + 1 > end.index) return null;
  return { before: close.slice(0, start + 1), raw: close.slice(start + 1, end.index), after: close.slice(end.index), quote: opening };
}

/** Maps index of an opening inline HTML tag to the index of its closing sibling. */
function pairHtml(nodes: PhrasingContent[]): Map<number, number> {
  const pairs = new Map<number, number>();
  const stack: { name: string; i: number }[] = [];
  nodes.forEach((node, i) => {
    if (node.type !== 'html') return;
    const tag = node.value.trim();
    const name = tagName(tag);
    if (!name) return;
    if (isClosingTag(tag)) {
      const idx = stack.map((s) => s.name).lastIndexOf(name);
      if (idx === -1) return;
      pairs.set(stack[idx].i, i);
      stack.length = idx;
    } else if (!isSelfClosing(tag) && !VOID_ELEMENTS.has(name)) {
      stack.push({ name, i });
    }
  });
  return pairs;
}

function isNoTranslateTag(tag: string): boolean {
  const name = tagName(tag);
  return (name !== null && NO_TRANSLATE_ELEMENTS.has(name)) || hasNoTranslateMarker(tag);
}

export function walkPhrasing(ctx: ExtractContext, nodes: PhrasingContent[], from: number, to: number, mb: MaskBuilder, deps: string[]) {
  const src = ctx.source;
  const htmlPairs = pairHtml(nodes);
  let pos = from;
  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i];
    const s = startOf(node);
    const e = endOf(node);
    if (s > pos) mb.source(src.slice(pos, s), 'markdown');
    pos = e;
    switch (node.type) {
      case 'text':
        mb.source(src.slice(s, e), 'markdown');
        break;
      case 'emphasis':
      case 'strong':
      case 'delete': {
        if (!node.children.length) {
          mb.placeholder(src.slice(s, e), src.slice(s, e));
          break;
        }
        const cs = startOf(node.children[0]);
        const ce = endOf(node.children[node.children.length - 1]);
        const n = mb.open(node.type, src.slice(s, cs), src.slice(ce, e), node.type);
        walkPhrasing(ctx, node.children, cs, ce, mb, deps);
        mb.close(n);
        break;
      }
      case 'inlineCode':
        mb.placeholder(src.slice(s, e), src.slice(s, e));
        break;
      case 'break': {
        const next = i + 1 < nodes.length ? startOf(nodes[i + 1]) : to;
        mb.placeholder(src.slice(s, next), 'line break', true);
        pos = next;
        break;
      }
      case 'link':
      case 'linkReference':
        linkLike(ctx, node, mb, deps);
        break;
      case 'image':
      case 'imageReference':
        imageLike(ctx, node, mb, deps);
        break;
      case 'html': {
        const tag = src.slice(s, e);
        const closeIdx = htmlPairs.get(i);
        if (closeIdx !== undefined) {
          const closeNode = nodes[closeIdx];
          if (isNoTranslateTag(tag)) {
            mb.placeholder(src.slice(s, endOf(closeNode)), src.slice(s, endOf(closeNode)));
          } else {
            const t = ctx.tagWithAttributes(tag, 'inline HTML');
            deps.push(...t.ids);
            const n = mb.open('html', t.render, src.slice(startOf(closeNode), endOf(closeNode)), tag);
            walkPhrasing(ctx, nodes.slice(i + 1, closeIdx), e, startOf(closeNode), mb, deps);
            mb.close(n);
          }
          pos = endOf(closeNode);
          i = closeIdx;
          break;
        }
        const t = ctx.tagWithAttributes(tag, 'inline HTML');
        deps.push(...t.ids);
        mb.placeholder(t.render, tag);
        break;
      }
      default:
        mb.placeholder(src.slice(s, e), src.slice(s, e));
    }
  }
  if (pos < to) mb.source(src.slice(pos, to), 'markdown');
}

function titleSegment(ctx: ExtractContext, close: string, deps: string[]): string | ((tm: TMap) => string) {
  const parts = splitTitle(close);
  if (!parts) return close;
  const mb = ctx.builder();
  mb.source(parts.raw, 'markdown');
  const seg = ctx.segment(mb, {
    kind: 'title',
    textContext: 'mdtitle',
    original: parts.raw,
    quote: parts.quote,
    embedded: true,
    note: 'link/image title (tooltip)',
  });
  if (!seg) return close;
  deps.push(seg.id);
  return (tm) => parts.before + ctx.render(seg, tm) + parts.after;
}

function linkLike(ctx: ExtractContext, node: Extract<PhrasingContent, { type: 'link' | 'linkReference' }>, mb: MaskBuilder, deps: string[]) {
  const src = ctx.source;
  const s = startOf(node);
  const e = endOf(node);
  if (!node.children.length || src[s] !== '[') {
    mb.placeholder(src.slice(s, e), src.slice(s, e));
    return;
  }
  const cs = startOf(node.children[0]);
  const ce = endOf(node.children[node.children.length - 1]);
  let close: string | ((tm: TMap) => string) = src.slice(ce, e);
  let ref: { label: string; close: string } | undefined;
  if (node.type === 'linkReference' && node.referenceType !== 'full') {
    // Translating the text of [label] / [label][] would break the reference; keep the label explicitly.
    ref = { label: src.slice(cs, ce), close: close as string };
    close = `][${ref.label}]`;
  } else if (node.type === 'link' && node.title != null) {
    close = titleSegment(ctx, close as string, deps);
  }
  const n = mb.open('link', src.slice(s, cs), close, node.type === 'link' ? `link to ${node.url}` : `link [${node.label ?? ''}]`);
  if (ref) Object.assign(mb.pairs.get(n)!, { refLabel: ref.label, closeOriginal: ref.close });
  walkPhrasing(ctx, node.children, cs, ce, mb, deps);
  mb.close(n);
}

function imageLike(ctx: ExtractContext, node: Extract<PhrasingContent, { type: 'image' | 'imageReference' }>, mb: MaskBuilder, deps: string[]) {
  const src = ctx.source;
  const s = startOf(node);
  const e = endOf(node);
  const altEnd = findLabelEnd(src, s + 1, e);
  const altRaw = altEnd === -1 ? '' : src.slice(s + 2, altEnd);
  if (altEnd === -1 || !/\p{L}/u.test(altRaw)) {
    mb.placeholder(src.slice(s, e), src.slice(s, e));
    return;
  }
  let close: string | ((tm: TMap) => string) = src.slice(altEnd, e);
  const refClose = close as string;
  if (node.type === 'imageReference' && node.referenceType !== 'full') close = `][${altRaw}]`;
  else if (node.type === 'image' && node.title != null) close = titleSegment(ctx, close as string, deps);
  const n = mb.open('image', src.slice(s, s + 2), close, node.type === 'image' ? `image ${node.url}` : 'image');
  if (node.type === 'imageReference' && node.referenceType !== 'full') Object.assign(mb.pairs.get(n)!, { refLabel: altRaw, closeOriginal: refClose });
  mb.source(altRaw, 'markdown');
  mb.close(n);
}

/** Order-independent signature of the inline structure (everything except text). */
export function inlineSignature(nodes: readonly Nodes[], cell = false): string {
  const items: string[] = [];
  const norm = (v: string) => (cell ? v.replace(/\\\|/g, '|') : v);
  const visit = (n: Nodes) => {
    switch (n.type) {
      case 'text':
        return;
      case 'inlineCode':
        items.push(`code:${norm(n.value).replace(/\\\|/g, '|')}`);
        return;
      case 'link':
        items.push(`link:${n.url}`);
        break;
      case 'image':
        items.push(`image:${n.url}`);
        return;
      case 'linkReference':
        items.push(`linkref:${n.identifier}`);
        break;
      case 'imageReference':
        items.push(`imageref:${n.identifier}`);
        return;
      case 'footnoteReference':
        items.push(`fn:${n.identifier}`);
        return;
      case 'html':
        items.push(`html:${tagSkeleton(n.value)}`);
        return;
      default:
        items.push(n.type);
    }
    if ('children' in n) for (const c of n.children) visit(c as Nodes);
  };
  for (const n of nodes) visit(n);
  return items.sort().join('\n');
}
