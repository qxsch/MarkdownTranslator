/** Attributes whose values are human-readable text and get translated. */
export const TRANSLATABLE_ATTRS = new Set(['alt', 'title', 'aria-label', 'aria-description', 'aria-placeholder', 'placeholder', 'label', 'summary', 'abbr']);

/** Elements whose content is never translated. */
export const NO_TRANSLATE_ELEMENTS = new Set(['script', 'style', 'pre', 'code', 'kbd', 'samp', 'var', 'tt', 'textarea', 'svg', 'math', 'template', 'noscript', 'iframe', 'object']);

export const VOID_ELEMENTS = new Set(['area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'param', 'source', 'track', 'wbr']);

export const INLINE_ELEMENTS = new Set([
  'a', 'abbr', 'b', 'bdi', 'bdo', 'br', 'cite', 'code', 'data', 'del', 'dfn', 'em', 'font', 'i', 'img', 'ins', 'kbd', 'label', 'mark', 'q', 's',
  'samp', 'small', 'span', 'strike', 'strong', 'sub', 'sup', 'time', 'tt', 'u', 'var', 'wbr', 'big', 'nobr', 'picture', 'source',
]);

export interface AttrValue {
  name: string;
  /** Offsets of the value (without quotes) relative to the tag string. */
  start: number;
  end: number;
  quote: '"' | "'" | '';
}

const ATTR = /[\s/]([A-Za-z_:@][-\w:.@]*)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;

export function tagName(tag: string): string | null {
  const m = /^<\/?([A-Za-z][A-Za-z0-9-]*)/.exec(tag);
  return m ? m[1].toLowerCase() : null;
}

export function isClosingTag(tag: string): boolean {
  return /^<\//.test(tag);
}

export function isSelfClosing(tag: string): boolean {
  return /\/\s*>$/.test(tag);
}

export function parseAttributes(tag: string): AttrValue[] {
  const nameEnd = /^<\/?[A-Za-z][A-Za-z0-9-]*/.exec(tag)?.[0].length ?? 0;
  const body = tag.slice(nameEnd);
  const out: AttrValue[] = [];
  ATTR.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = ATTR.exec(body))) {
    const name = m[1].toLowerCase();
    const raw = m[0];
    let value: string | undefined;
    let quote: AttrValue['quote'] = '';
    if (m[2] !== undefined) (value = m[2]), (quote = '"');
    else if (m[3] !== undefined) (value = m[3]), (quote = "'");
    else if (m[4] !== undefined) value = m[4];
    if (value === undefined) continue;
    const valueEndInRaw = raw.length - (quote ? 1 : 0);
    const valueStartInRaw = valueEndInRaw - value.length;
    const base = nameEnd + m.index;
    out.push({ name, start: base + valueStartInRaw, end: base + valueEndInRaw, quote });
  }
  return out;
}

export function hasNoTranslateMarker(tag: string): boolean {
  return /\stranslate\s*=\s*["']?no\b/i.test(tag) || /\sclass\s*=\s*["'][^"']*\bnotranslate\b/i.test(tag);
}

/** Attribute values stripped of translatable text, for structural comparisons. */
export function tagSkeleton(tag: string): string {
  let out = '';
  let pos = 0;
  for (const a of parseAttributes(tag)) {
    if (!TRANSLATABLE_ATTRS.has(a.name)) continue;
    out += tag.slice(pos, a.start) + '…';
    pos = a.end;
  }
  return out + tag.slice(pos);
}
