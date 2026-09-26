import { parse as parseYaml } from 'yaml';
import type { Nodes } from 'mdast';
import { parseMarkdown, startOf, endOf, type ParseOptions } from './parse.js';
import { inlineSignature, jsxSkeleton } from './inline.js';
import { tagSkeleton } from './htmlAttrs.js';
import { FRONTMATTER_KEYS, FRONTMATTER_LIST_KEYS } from './frontmatter.js';
import { codeFingerprint } from '../code/parsers.js';
import type { Extraction, RenderOptions, TMap } from './types.js';

export function applyTranslations(ex: Extraction, tm: TMap, opts?: RenderOptions): string {
  let out = '';
  let pos = 0;
  for (const r of ex.replacements) {
    out += ex.source.slice(pos, r.start) + r.render(tm, opts);
    pos = r.end;
  }
  return out + ex.source.slice(pos);
}

export function htmlSkeleton(html: string): string {
  return (html.match(/<!--[\s\S]*?-->|<[^<>]*>/g) ?? []).map((t) => (t.startsWith('<!--') ? t : tagSkeleton(t))).join('');
}

function maskYaml(data: unknown): unknown {
  if (Array.isArray(data)) return data.map(maskYaml);
  if (!data || typeof data !== 'object') return data;
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(data)) {
    const key = k.toLowerCase();
    if (FRONTMATTER_KEYS.has(key) && typeof v === 'string') out[k] = '…';
    else if (FRONTMATTER_LIST_KEYS.has(key) && Array.isArray(v)) out[k] = v.map((x) => (typeof x === 'string' ? '…' : maskYaml(x)));
    else out[k] = maskYaml(v);
  }
  return out;
}

function yamlSkeleton(value: string): string {
  try {
    return JSON.stringify(maskYaml(parseYaml(value)));
  } catch {
    return `invalid-yaml`;
  }
}

/** Heading anchors inserted by preserveAnchors (HTML in Markdown, JSX in MDX). */
function isInjectedAnchor(c: Nodes): boolean {
  if (c.type === 'html') return /^(?:<a\s+id="[^"]*">|<\/a>)$/.test(c.value);
  if (c.type === 'mdxJsxTextElement') return c.name === 'a' && !c.children.length && c.attributes.length === 1;
  return false;
}

export interface SkeletonItem {
  sig: string;
  start: number;
  end: number;
}

/** Structural fingerprint of a Markdown document: everything except translatable text. */
export function skeleton(text: string, opts: ParseOptions = {}): SkeletonItem[] {
  const tree = parseMarkdown(text, opts);
  const out: SkeletonItem[] = [];
  const push = (n: Nodes, sig: string) => out.push({ sig, start: startOf(n), end: endOf(n) });
  const visit = (n: Nodes) => {
    switch (n.type) {
      case 'paragraph':
        push(n, `p\n${inlineSignature(n.children)}`);
        return;
      case 'heading':
        push(n, `h${n.depth}\n${inlineSignature(n.children.filter((c) => !isInjectedAnchor(c)))}`);
        return;
      case 'tableCell':
        push(n, `td\n${inlineSignature(n.children, true)}`);
        return;
      case 'code':
        push(n, `code:${n.lang ?? ''}:${n.meta ?? ''}:${(n.lang && codeFingerprint(n.lang, n.value)) ?? n.value}`);
        return;
      case 'html':
        push(n, `html:${htmlSkeleton(n.value)}`);
        return;
      case 'yaml':
        push(n, `yaml:${yamlSkeleton(n.value)}`);
        return;
      case 'definition':
        push(n, `def:${n.identifier}:${n.url}`);
        return;
      case 'list':
        push(n, `list:${n.ordered}:${n.start ?? ''}`);
        break;
      case 'listItem':
        push(n, `li:${n.checked ?? ''}`);
        break;
      case 'table':
        push(n, `table:${(n.align ?? []).join(',')}`);
        break;
      case 'footnoteDefinition':
        push(n, `fndef:${n.identifier}`);
        break;
      case 'math':
        push(n, `math:${n.value}`);
        return;
      case 'containerDirective':
        push(n, `dir:${n.name}:${JSON.stringify(n.attributes ?? {})}`);
        break;
      case 'leafDirective':
        push(n, `leaf:${n.name}:${JSON.stringify(n.attributes ?? {})}\n${inlineSignature(n.children)}`);
        return;
      case 'mdxjsEsm':
      case 'mdxFlowExpression':
        push(n, `${n.type}:${n.value}`);
        return;
      case 'mdxJsxFlowElement':
        push(n, `jsx:${jsxSkeleton(n)}`);
        break;
      case 'root':
        break;
      default:
        push(n, n.type);
    }
    if ('children' in n) for (const c of n.children) visit(c as Nodes);
  };
  visit(tree);
  return out;
}

/** Index of the first differing skeleton item, or -1 when identical. */
export function compareSkeletons(a: SkeletonItem[], b: SkeletonItem[]): number {
  const n = Math.max(a.length, b.length);
  for (let i = 0; i < n; i++) if (a[i]?.sig !== b[i]?.sig) return i;
  return -1;
}
