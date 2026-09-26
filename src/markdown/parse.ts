import { fromMarkdown, type Extension as MdastExtension } from 'mdast-util-from-markdown';
import { gfm } from 'micromark-extension-gfm';
import { gfmFromMarkdown } from 'mdast-util-gfm';
import { frontmatter } from 'micromark-extension-frontmatter';
import { frontmatterFromMarkdown } from 'mdast-util-frontmatter';
import { math } from 'micromark-extension-math';
import { mathFromMarkdown } from 'mdast-util-math';
import { directive } from 'micromark-extension-directive';
import { directiveFromMarkdown } from 'mdast-util-directive';
import { mdxjs } from 'micromark-extension-mdxjs';
import { mdxFromMarkdown } from 'mdast-util-mdx';
import type { Extension } from 'micromark-util-types';
import type { Root, Nodes } from 'mdast';
import { findDollarMath } from './mathSpans.js';

export interface ParseOptions {
  /** Parse as MDX (JSX, ESM, expressions); used for .mdx files. */
  mdx?: boolean;
  /** Parse `$x$` as inline math (Pandoc rules plus a formula check, so prices stay prose). */
  mathSingleDollar?: boolean;
}

// Only block directives (:::note / ::leaf); text directives would turn prose like "foo:bar" into syntax.
const blockDirectives: Extension = { flow: directive().flow };

// The GFM autolink tree transform creates nodes without source positions (e.g. "[!VIDEO https://...]"); the URLs
// stay text and are protected by the masking rules. Syntax-level autolinks keep their positions.
const gfmMdast = () => gfmFromMarkdown().map((e) => ({ ...e, transforms: [] }));

export function parseMarkdown(text: string, opts: ParseOptions = {}): Root {
  const extensions: Extension[] = [gfm(), frontmatter(['yaml', 'toml']), math({ singleDollarTextMath: false }), blockDirectives];
  const mdastExtensions: (MdastExtension | MdastExtension[])[] = [...gfmMdast(), frontmatterFromMarkdown(['yaml', 'toml']), mathFromMarkdown(), directiveFromMarkdown()];
  if (opts.mdx) {
    extensions.push(mdxjs());
    mdastExtensions.push(mdxFromMarkdown());
  }
  const tree = fromMarkdown(text, { extensions, mdastExtensions });
  if (opts.mathSingleDollar) splitDollarMath(tree, text);
  return tree;
}

/** Turns `$...$` spans inside text nodes into inlineMath nodes (see mathSpans.ts for the rules). */
function splitDollarMath(tree: Root, src: string) {
  const lineStarts = [0];
  for (let i = 0; i < src.length; i++) if (src[i] === '\n') lineStarts.push(i + 1);
  const point = (offset: number) => {
    let lo = 0;
    let hi = lineStarts.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (lineStarts[mid] <= offset) lo = mid;
      else hi = mid - 1;
    }
    return { line: lo + 1, column: offset - lineStarts[lo] + 1, offset };
  };
  const visit = (node: Nodes) => {
    if (!('children' in node)) return;
    const children = node.children as Nodes[];
    for (let k = 0; k < children.length; k++) {
      const c = children[k];
      if (c.type !== 'text' || c.position?.start.offset === undefined || c.position.end.offset === undefined) {
        visit(c);
        continue;
      }
      const s = c.position.start.offset;
      const raw = src.slice(s, c.position.end.offset);
      const spans = findDollarMath(raw);
      if (!spans.length) continue;
      const parts: Nodes[] = [];
      let pos = 0;
      const text = (a: number, b: number) => parts.push({ type: 'text', value: raw.slice(a, b), position: { start: point(s + a), end: point(s + b) } });
      for (const m of spans) {
        if (m.start > pos) text(pos, m.start);
        parts.push({ type: 'inlineMath', value: raw.slice(m.start + 1, m.end - 1), position: { start: point(s + m.start), end: point(s + m.end) } });
        pos = m.end;
      }
      if (pos < raw.length) text(pos, raw.length);
      children.splice(k, 1, ...parts);
      k += parts.length - 1;
    }
  };
  visit(tree);
}

export function startOf(node: Nodes): number {
  const o = node.position?.start.offset;
  if (o === undefined) throw new Error(`node ${node.type} has no position`);
  return o;
}

export function endOf(node: Nodes): number {
  const o = node.position?.end.offset;
  if (o === undefined) throw new Error(`node ${node.type} has no position`);
  return o;
}
