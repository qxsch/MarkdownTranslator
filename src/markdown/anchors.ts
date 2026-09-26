import GithubSlugger from 'github-slugger';
import type { Heading, Nodes } from 'mdast';
import { parseMarkdown, startOf, type ParseOptions } from './parse.js';

interface HeadingInfo {
  text: string;
  contentStart: number;
  hasExplicitId: boolean;
}

function textOf(n: Nodes): string {
  if (n.type === 'text' || n.type === 'inlineCode') return n.value;
  if (n.type === 'html' || n.type === 'image' || n.type === 'imageReference') return '';
  if ('children' in n) return (n.children as Nodes[]).map(textOf).join('');
  return '';
}

function headings(text: string, opts: ParseOptions): HeadingInfo[] {
  const out: HeadingInfo[] = [];
  const visit = (n: Nodes) => {
    if (n.type === 'heading') {
      const h = n as Heading;
      if (!h.children.length) return;
      const plain = textOf(h);
      out.push({
        text: plain,
        contentStart: startOf(h.children[0]),
        hasExplicitId:
          h.children.some((c) => (c.type === 'html' && /<a\s[^>]*\b(?:id|name)\s*=/i.test(c.value)) || (c.type === 'mdxJsxTextElement' && c.name === 'a' && c.attributes.some((a) => a.type === 'mdxJsxAttribute' && (a.name === 'id' || a.name === 'name')))) ||
          /\{#[\w-]+\}\s*$/.test(plain),
      });
      return;
    }
    if ('children' in n) for (const c of n.children as Nodes[]) visit(c);
  };
  visit(parseMarkdown(text, opts));
  return out;
}

/**
 * Keeps links to the original heading anchors (#getting-started) working after headings were translated,
 * by inserting an empty HTML anchor with the original slug at the start of each changed heading.
 * The anchor has no text, so it does not change the new heading's own slug.
 */
export function preserveAnchors(source: string, output: string, opts: ParseOptions = {}): { text: string; added: string[] } {
  const a = headings(source, opts);
  const b = headings(output, opts);
  if (a.length !== b.length) return { text: output, added: [] };
  const oldSlugger = new GithubSlugger();
  const newSlugger = new GithubSlugger();
  const inserts: { at: number; text: string; slug: string }[] = [];
  for (let i = 0; i < a.length; i++) {
    const oldSlug = oldSlugger.slug(a[i].text);
    const newSlug = newSlugger.slug(b[i].text);
    if (oldSlug && oldSlug !== newSlug && !b[i].hasExplicitId && !a[i].hasExplicitId) {
      inserts.push({ at: b[i].contentStart, text: `<a id="${oldSlug}"></a>`, slug: oldSlug });
    }
  }
  let text = output;
  for (const ins of [...inserts].sort((x, y) => y.at - x.at)) text = text.slice(0, ins.at) + ins.text + text.slice(ins.at);
  return { text, added: inserts.map((i) => i.slug) };
}
