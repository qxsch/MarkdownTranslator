import type { Nodes, Heading, Paragraph, TableCell } from 'mdast';
import { ExtractContext } from './context.js';
import { inlineSignature, walkPhrasing } from './inline.js';
import { processHtmlBlock } from './html.js';
import { processFrontmatter } from './frontmatter.js';
import { processCodeBlock } from '../code/comments.js';
import { lineStartOf, maxLineWidth } from './lineMap.js';
import { endOf, parseMarkdown, startOf } from './parse.js';
import type { Extraction, SegmentKind } from './types.js';

export function extract(input: string, doNotTranslate: readonly string[] = []): Extraction {
  const bom = input.startsWith('\uFEFF') ? '\uFEFF' : '';
  const source = bom ? input.slice(1) : input;
  const eol = source.includes('\r\n') ? '\r\n' : '\n';
  const ctx = new ExtractContext(source, doNotTranslate, eol);
  const tree = parseMarkdown(source);
  const definitions: string[] = [];
  let frontmatterFormality: 'formal' | 'informal' | undefined;

  const prose = (node: Paragraph | Heading | TableCell, kind: SegmentKind, note: string) => {
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

  const visit = (node: Nodes, where: string) => {
    switch (node.type) {
      case 'root':
        for (const c of node.children) visit(c, 'paragraph');
        return;
      case 'blockquote':
        for (const c of node.children) visit(c, 'paragraph in a blockquote');
        return;
      case 'list':
        for (const c of node.children) visit(c, 'list item');
        return;
      case 'listItem':
        for (const c of node.children) visit(c, where);
        return;
      case 'footnoteDefinition':
        definitions.push(`[^${node.label ?? node.identifier}]: x`);
        for (const c of node.children) visit(c, 'footnote');
        return;
      case 'table':
        node.children.forEach((row, i) => row.children.forEach((cell) => prose(cell, 'cell', i === 0 ? 'table header cell' : 'table cell')));
        return;
      case 'paragraph':
        prose(node, 'paragraph', where);
        return;
      case 'heading':
        prose(node, 'heading', `heading level ${node.depth}`);
        return;
      case 'html':
        processHtmlBlock(ctx, node);
        return;
      case 'code':
        processCodeBlock(ctx, node);
        return;
      case 'yaml':
        frontmatterFormality = processFrontmatter(ctx, node);
        return;
      case 'definition': {
        const s = startOf(node);
        const raw = source.slice(s, endOf(node));
        definitions.push(raw);
        const m = node.title != null ? /^(\[(?:\\.|[^\]\\])*\]:\s*(?:<[^>\n]*>|\S+)\s+)(["'(])([\s\S]*)["')][ \t]*$/.exec(raw) : null;
        if (m) {
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
  visit(tree, 'paragraph');

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
  };
}
