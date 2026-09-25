import type { Yaml } from 'mdast';
import type { ExtractContext } from './context.js';
import { endOf, startOf } from './parse.js';

/** Front matter keys whose values are prose. */
export const FRONTMATTER_KEYS = new Set([
  'title', 'subtitle', 'description', 'summary', 'excerpt', 'abstract', 'caption', 'heading', 'lead', 'teaser', 'tagline',
  'seo_title', 'seotitle', 'seo_description', 'og_title', 'og_description', 'twitter_title', 'twitter_description', 'sidebar_label', 'nav_title', 'linktitle', 'menu_title',
]);

const FORMALITY_KEYS = /^(?:formality|translation_formality|translation-formality|tone)$/i;

function needsQuotes(s: string): boolean {
  return /^[\s\-?:,[\]{}#&*!|>'"%@`]|:\s|\s#|:$|^\s|\s$|^(?:true|false|yes|no|on|off|null|~|[-+]?\d[\d_.]*(?:e[-+]?\d+)?)$/i.test(s);
}

const dq = (s: string) => `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;

export function processFrontmatter(ctx: ExtractContext, node: Yaml): 'formal' | 'informal' | undefined {
  const src = ctx.source;
  const s = startOf(node);
  const e = endOf(node);
  const firstNl = src.indexOf('\n', s);
  if (firstNl === -1 || firstNl >= e) return undefined;
  let pos = firstNl + 1;
  let formality: 'formal' | 'informal' | undefined;
  const end = e;
  while (pos < end) {
    let nl = src.indexOf('\n', pos);
    if (nl === -1 || nl > end) nl = end;
    const line = src.slice(pos, nl).replace(/\r$/, '');
    const m = /^([A-Za-z_][\w.-]*)([ \t]*:[ \t]+)(.*?)[ \t]*$/.exec(line);
    if (m) {
      const key = m[1];
      const raw = m[3];
      const valueStart = pos + m[1].length + m[2].length;
      if (FORMALITY_KEYS.test(key)) {
        const v = raw.replace(/^["']|["']$/g, '').toLowerCase();
        if (v === 'formal' || v === 'informal') formality = v;
      } else if (FRONTMATTER_KEYS.has(key.toLowerCase()) && raw && !/^[|>[{&*!%@`]/.test(raw) && !/\s#/.test(raw.replace(/^(["']).*\1$/, ''))) {
        valueSegment(ctx, key, raw, valueStart);
      }
    }
    pos = nl + 1;
  }
  return formality;
}

function valueSegment(ctx: ExtractContext, key: string, raw: string, start: number) {
  let inner: string;
  let finalize: (s: string) => string;
  if (/^".*"$/.test(raw) && raw.length >= 2) {
    inner = raw.slice(1, -1);
    if (/\\[^"\\]/.test(inner)) return;
    inner = inner.replace(/\\(["\\])/g, '$1');
    finalize = dq;
  } else if (/^'.*'$/.test(raw) && raw.length >= 2) {
    inner = raw.slice(1, -1).replace(/''/g, "'");
    finalize = (s) => `'${s.replace(/'/g, "''")}'`;
  } else if (/^["']/.test(raw)) {
    return;
  } else {
    inner = raw;
    finalize = (s) => (needsQuotes(s) ? dq(s) : s);
  }
  const mb = ctx.builder();
  mb.source(inner, 'none');
  const seg = ctx.segment(mb, { kind: 'frontmatter', textContext: 'yaml', original: raw, note: `front matter "${key}"`, finalize });
  if (seg) ctx.replace(start, start + raw.length, seg);
}
