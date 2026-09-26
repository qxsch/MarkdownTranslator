import type { Nodes, Heading, Paragraph, TableCell, RootContent, PhrasingContent } from 'mdast';
import { toString } from 'mdast-util-to-string';
import { ExtractContext, type ExtractOptions } from './context.js';
import { inlineSignature, walkPhrasing } from './inline.js';
import { processHtmlBlock } from './html.js';
import { processFrontmatter } from './frontmatter.js';
import { processCodeBlock } from '../code/comments.js';
import { lineStartOf, maxLineWidth } from './lineMap.js';
import { endOf, parseMarkdown, startOf } from './parse.js';
import type { Extraction, SegmentKind } from './types.js';

const clip = (s: string, n = 80) => {
  const one = s.replace(/\s+/g, ' ').trim();
  return one.length > n ? `${one.slice(0, n - 1)}\u2026` : one;
};

/** End of a JSX opening tag starting at `start` (quotes and {expressions} may contain `>`). */
function jsxTagEnd(src: string, start: number, limit: number): number {
  let depth = 0;
  let quote = '';
  for (let i = start; i < limit; i++) {
    const c = src[i];
    if (quote) {
      if (c === quote) quote = '';
    } else if (c === '"' || c === "'") quote = c;
    else if (c === '{') depth++;
    else if (c === '}') depth--;
    else if (c === '>' && depth === 0) return i + 1;
  }
  return -1;
}

export function extract(input: string, doNotTranslate: readonly string[] = [], options: ExtractOptions = {}): Extraction {
  const bom = input.startsWith('\uFEFF') ? '\uFEFF' : '';
  const source = bom ? input.slice(1) : input;
  const eol = source.includes('\r\n') ? '\r\n' : '\n';
  const parseOptions = options.parse ?? {};
  const ctx = new ExtractContext(source, doNotTranslate, eol, options);
  const tree = parseMarkdown(source, parseOptions);
  const definitions: string[] = [];
  let frontmatterFormality: 'formal' | 'informal' | undefined;
  const headings: string[] = [];

  /** Where a segment sits: section path plus local context (table column, list lead-in, component). */
  const setStructure = (...local: (string | undefined)[]) => {
    const section = headings.filter(Boolean).join(' > ');
    const parts = [section ? `section "${clip(section, 160)}"` : undefined, ...local].filter(Boolean);
    ctx.structure = parts.length ? parts.join('; ') : undefined;
  };

  const prose = (node: Paragraph | Heading | TableCell | { children: PhrasingContent[] }, kind: SegmentKind, note: string) => {
    if (!node.children.length) return;
    const from = startOf(node.children[0]);
    const to = endOf(node.children[node.children.length - 1]);
    const mb = ctx.builder();
    const deps: string[] = [];
    walkPhrasing(ctx, node.children, from, to, mb, deps);
    const wrap =
      kind === 'paragraph' && mb.softBreak
        ? { width: Math.max(maxLineWidth(source, from, to), 40), firstColumn: from - lineStartOf(source, from), prefix: mb.softBreak.prefix, eol: mb.softBreak.eol }
        : undefined;
    const seg = ctx.segment(mb, {
      kind,
      textContext: kind === 'cell' ? 'cell' : 'markdown',
      original: source.slice(from, to),
      note,
      wrap,
      dependents: deps,
      force: true,
      inlineSignature: inlineSignature(node.children, kind === 'cell'),
    });
    if (seg) ctx.replace(from, to, seg);
  };

  const children = (parent: { children: RootContent[] }, where: string, local: string[]) => {
    parent.children.forEach((c, i) => {
      const prev = parent.children[i - 1];
      const leadIn = c.type === 'list' && prev?.type === 'paragraph' ? `list introduced by "${clip(toString(prev))}"` : undefined;
      visit(c, where, leadIn ? [...local, leadIn] : local);
    });
  };

  const visit = (node: Nodes, where: string, local: string[]) => {
    switch (node.type) {
      case 'root':
        children(node, 'paragraph', local);
        return;
      case 'blockquote':
        children(node, 'paragraph in a blockquote', local);
        return;
      case 'list':
        children(node, 'list item', local);
        return;
      case 'listItem':
        children(node, where, local);
        return;
      case 'footnoteDefinition':
        definitions.push(`[^${node.label ?? node.identifier}]: x`);
        children(node, 'footnote', [...local, 'footnote']);
        return;
      case 'containerDirective':
        children(node, 'paragraph', [...local, `inside "${node.name}" admonition/directive`]);
        return;
      case 'leafDirective':
        setStructure(...local, `"${node.name}" directive label`);
        prose(node, 'label', `"${node.name}" directive label`);
        return;
      case 'mdxJsxFlowElement': {
        const s = startOf(node);
        const e = endOf(node);
        const tagEnd = jsxTagEnd(source, s, e);
        if (tagEnd !== -1) {
          setStructure(...local);
          const tag = source.slice(s, tagEnd);
          const t = ctx.tagWithAttributes(tag, `<${node.name}> component`);
          if (t.ids.length) {
            const mb = ctx.builder();
            mb.placeholder(t.render, tag);
            const seg = ctx.segment(mb, { kind: 'html', textContext: 'html', original: tag, note: `<${node.name}> component attributes`, dependents: t.ids, force: true });
            if (seg) ctx.replace(s, tagEnd, seg);
          }
        }
        children(node, 'paragraph', [...local, `inside <${node.name}> component`]);
        return;
      }
      case 'table': {
        const header = node.children[0]?.children.map((c) => clip(toString(c), 40)) ?? [];
        node.children.forEach((row, r) => {
          const rowLabel = row.children[0] ? clip(toString(row.children[0]), 40) : '';
          row.children.forEach((cell, c) => {
            const pos =
              r === 0
                ? `table header; columns: ${header.map((h) => `"${h}"`).join(', ')}`
                : `table column "${header[c] ?? ''}"${c > 0 && rowLabel ? `, row "${rowLabel}"` : ''}`;
            setStructure(...local, pos);
            prose(cell, 'cell', r === 0 ? 'table header cell' : 'table cell');
          });
        });
        return;
      }
      case 'paragraph':
        setStructure(...local, node.data && (node.data as { directiveLabel?: boolean }).directiveLabel ? 'admonition title' : undefined);
        prose(node, 'paragraph', where);
        return;
      case 'heading': {
        headings.length = node.depth - 1;
        setStructure(...local);
        prose(node, 'heading', `heading level ${node.depth}`);
        headings[node.depth - 1] = toString(node);
        return;
      }
      case 'html':
        setStructure(...local);
        processHtmlBlock(ctx, node);
        return;
      case 'code':
        if (options.codeComments === false) return;
        setStructure(...local, node.lang ? `${node.lang} code block` : undefined);
        processCodeBlock(ctx, node);
        return;
      case 'yaml':
        frontmatterFormality = processFrontmatter(ctx, node, options.frontMatter !== false);
        return;
      case 'definition': {
        const s = startOf(node);
        const raw = source.slice(s, endOf(node));
        definitions.push(raw);
        const m = node.title != null ? /^(\[(?:\\.|[^\]\\])*\]:\s*(?:<[^>\n]*>|\S+)\s+)(["'(])([\s\S]*)["')][ \t]*$/.exec(raw) : null;
        if (m) {
          setStructure(...local);
          const mb = ctx.builder();
          mb.source(m[3], 'markdown');
          const seg = ctx.segment(mb, { kind: 'title', textContext: 'mdtitle', original: m[3], quote: m[2], note: 'link title (tooltip)' });
          if (seg) ctx.replace(s + m[1].length + 1, s + m[1].length + 1 + m[3].length, seg);
        }
        return;
      }
      default:
        return;
    }
  };
  visit(tree, 'paragraph', []);

  const replacements = [...ctx.replacements].sort((a, b) => a.start - b.start);
  for (let i = 1; i < replacements.length; i++) {
    if (replacements[i].start < replacements[i - 1].end) throw new Error(`overlapping replacements at ${replacements[i].start}`);
  }
  return {
    source,
    bom,
    eol,
    segments: ctx.segments,
    byId: ctx.byId,
    replacements,
    notes: ctx.notes,
    definitionsText: definitions.join('\n'),
    frontmatterFormality,
    parseOptions,
  };
}
