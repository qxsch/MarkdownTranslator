import type { Yaml } from 'mdast';
import { isMap, isScalar, isSeq, parseDocument, type Node as YamlNode, type Scalar } from 'yaml';
import type { ExtractContext } from './context.js';
import { maxLineWidth } from './lineMap.js';
import { endOf, startOf } from './parse.js';

/** Front matter keys (at any nesting depth) whose string values are prose. */
export const FRONTMATTER_KEYS = new Set([
  'title', 'subtitle', 'titlesuffix', 'description', 'summary', 'excerpt', 'abstract', 'caption', 'heading', 'lead', 'teaser', 'tagline',
  'seo_title', 'seotitle', 'seo_description', 'og_title', 'og_description', 'twitter_title', 'twitter_description', 'sidebar_label', 'nav_title', 'linktitle', 'menu_title',
]);

/** Front matter keys whose list items are prose. */
export const FRONTMATTER_LIST_KEYS = new Set(['keywords']);

const FORMALITY_KEYS = /^(?:formality|translation_formality|translation-formality|tone)$/i;

function needsQuotes(s: string): boolean {
  return /^[\s\-?:,[\]{}#&*!|>'"%@`]|:\s|\s#|:$|^\s|\s$|^(?:true|false|yes|no|on|off|null|~|[-+]?\d[\d_.]*(?:e[-+]?\d+)?)$/i.test(s);
}

const dq = (s: string) => JSON.stringify(s);

export function processFrontmatter(ctx: ExtractContext, node: Yaml, translate = true): 'formal' | 'informal' | undefined {
  const src = ctx.source;
  const s = startOf(node);
  const e = endOf(node);
  const firstNl = src.indexOf('\n', s);
  if (firstNl === -1 || firstNl >= e) return undefined;
  const base = firstNl + 1;
  const value = node.value;
  if (src.slice(base, base + value.length) !== value) {
    ctx.notes.push('front matter left untranslated (could not map it to the source)');
    return undefined;
  }
  const doc = parseDocument(value);
  if (doc.errors.length || !isMap(doc.contents)) {
    if (doc.errors.length) ctx.notes.push(`front matter left untranslated (YAML error: ${doc.errors[0].message.split('\n')[0]})`);
    return undefined;
  }
  let formality: 'formal' | 'informal' | undefined;
  const previous = ctx.structure;

  const visitMap = (map: YamlNode, path: string[]) => {
    if (!isMap(map)) return;
    for (const pair of map.items) {
      if (!isScalar(pair.key)) continue;
      const key = String(pair.key.value);
      const lower = key.toLowerCase();
      const v = pair.value as YamlNode | null;
      if (!v) continue;
      if (path.length === 0 && FORMALITY_KEYS.test(key) && isScalar(v)) {
        const f = String(v.value).toLowerCase();
        if (f === 'formal' || f === 'informal') formality = f;
      } else if (!translate) {
        continue;
      } else if (FRONTMATTER_KEYS.has(lower) && isScalar(v) && typeof v.value === 'string') {
        scalarSegments(ctx, v, base, [...path, key].join('.'));
      } else if (FRONTMATTER_LIST_KEYS.has(lower) && isSeq(v)) {
        for (const item of v.items) if (isScalar(item) && typeof item.value === 'string') scalarSegments(ctx, item, base, [...path, key].join('.'));
      } else if (isMap(v)) {
        visitMap(v, [...path, key]);
      } else if (isSeq(v)) {
        for (const item of v.items) if (isMap(item)) visitMap(item, [...path, key]);
      }
    }
  };
  visitMap(doc.contents, []);
  ctx.structure = previous;
  return formality;
}

function scalarSegments(ctx: ExtractContext, node: Scalar, base: number, keyPath: string) {
  const src = ctx.source;
  const [start, end] = node.range!;
  const from = base + start;
  const to = base + end;
  const raw = src.slice(from, to).replace(/\s+$/, '');
  ctx.structure = `front matter "${keyPath}"`;
  const note = `front matter "${keyPath}"`;
  const add = (text: string, rFrom: number, rTo: number, finalize?: (s: string) => string, wrap?: boolean) => {
    const mb = ctx.builder();
    mb.source(text, 'none');
    const spec =
      wrap && mb.softBreak
        ? { width: Math.max(maxLineWidth(src, rFrom, rTo), 40), firstColumn: rFrom - (src.lastIndexOf('\n', rFrom - 1) + 1), prefix: mb.softBreak.prefix, eol: mb.softBreak.eol }
        : undefined;
    const seg = ctx.segment(mb, { kind: 'frontmatter', textContext: 'yaml', original: src.slice(rFrom, rTo), note, finalize, wrap: spec });
    if (seg) ctx.replace(rFrom, rTo, seg);
  };

  switch (node.type) {
    case 'QUOTE_DOUBLE':
      add(node.value as string, from, from + raw.length, dq);
      return;
    case 'QUOTE_SINGLE':
      add(node.value as string, from, from + raw.length, (s) => `'${s.replace(/'/g, "''")}'`);
      return;
    case 'PLAIN':
      add(node.value as string, from, from + raw.length, (s) => (needsQuotes(s) ? dq(s) : s));
      return;
    case 'BLOCK_FOLDED':
    case 'BLOCK_LITERAL': {
      const headerEnd = raw.indexOf('\n');
      if (headerEnd === -1) return;
      const contentFrom = from + headerEnd + 1;
      const content = src.slice(contentFrom, from + raw.length);
      if (node.type === 'BLOCK_LITERAL') {
        // Line breaks are meaningful in literal blocks: every line is its own segment.
        let pos = contentFrom;
        for (const line of content.split('\n')) {
          const body = line.replace(/\r$/, '');
          const indent = /^[ \t]*/.exec(body)![0].length;
          const text = body.slice(indent).replace(/[ \t]+$/, '');
          if (text) add(text, pos + indent, pos + indent + text.length);
          pos += line.length + 1;
        }
        return;
      }
      // Folded: blank lines separate paragraphs; each paragraph is re-wrapped with the block indentation.
      const re = /(?:^|\n)((?:[ \t]*\S[^\n]*(?:\n|$))+)/g;
      for (const m of content.matchAll(re)) {
        const block = m[1].replace(/\s+$/, '');
        const offset = m.index! + (m[0].length - m[1].length);
        const indent = /^[ \t]*/.exec(block)![0].length;
        const pFrom = contentFrom + offset + indent;
        add(block.slice(indent), pFrom, pFrom + block.length - indent, undefined, true);
      }
      return;
    }
  }
}
