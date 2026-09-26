import { findProtected } from './protect.js';
import type { Pair, PairKind, Placeholder, Segment, TMap, TagGroup, TextContext } from '../markdown/types.js';

export const xmlEscape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
export const xmlUnescape = (s: string) =>
  s.replace(/&(lt|gt|quot|apos|amp);/g, (_, e: string) => ({ lt: '<', gt: '>', quot: '"', apos: "'", amp: '&' })[e]!);

const MD_ESCAPE = /\\[!-/:-@[-`{-~]/g;
const ENTITY = /&(?:[A-Za-z][A-Za-z0-9]{1,31}|#\d{1,7}|#[xX][0-9a-fA-F]{1,6});/g;
/** Line break inside running text plus the container prefix (blockquote markers / indentation) that follows it. */
const SOFT_BREAK = /[ \t]*(\r?\n)((?:[ \t]*>)*[ \t]*)/g;
/** Human-readable attribute values inside shortcodes and directive lines. */
const ATTR_VALUE = /\b(alt|alt-text|title|caption|label|summary)(\s*=\s*)(["'])(.*?)\3/g;
/** Lines that are structure on their own: `:::note Title`, `:::image ... :::`, `:::`, `[!NOTE]`, `{{% notice %}}`. */
const FENCE_LINE = /^(?::{3,}|\[![A-Za-z]+\]\s*$|\{\{[<%][^\n]*[>%]\}\}\s*$)/;

export type EscapeMode = 'markdown' | 'html' | 'none';

export interface SoftBreakInfo {
  eol: string;
  prefix: string;
}

export class MaskBuilder {
  private parts: string[] = [];
  readonly placeholders = new Map<number, Placeholder>();
  readonly pairs = new Map<number, Pair>();
  readonly tagGroups: TagGroup[] = [];
  softBreak: SoftBreakInfo | null = null;
  private n = 0;
  /** Plain (decoded) source text, used to decide which markup characters need escaping on output. */
  sourceText = '';

  constructor(private readonly dnt: readonly string[]) {}

  get masked(): string {
    return this.parts.join('');
  }

  text(t: string) {
    if (!t) return;
    this.sourceText += t;
    this.parts.push(xmlEscape(t));
  }

  placeholder(raw: string | ((tm: TMap) => string), hint: string, hardBreak = false): number {
    const n = ++this.n;
    const render = typeof raw === 'string' ? () => raw : raw;
    this.placeholders.set(n, { n, hint: truncate(hint), render, hardBreak });
    this.parts.push(`<x${n}/>`);
    return n;
  }

  open(kind: PairKind, open: string | ((tm: TMap) => string), close: string | ((tm: TMap) => string), hint: string): number {
    const n = ++this.n;
    this.pairs.set(n, {
      n,
      kind,
      hint: truncate(hint),
      open: typeof open === 'string' ? () => open : open,
      close: typeof close === 'string' ? () => close : close,
    });
    this.parts.push(`<g${n}>`);
    return n;
  }

  close(n: number) {
    this.parts.push(`</g${n}>`);
  }

  /** Adds raw source text: protects identifiers/paths/escapes and turns line breaks into spaces. */
  source(raw: string, mode: EscapeMode) {
    const lines: string[] = [];
    const breaks: RegExpExecArray[] = [];
    let last = 0;
    SOFT_BREAK.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = SOFT_BREAK.exec(raw))) {
      lines.push(raw.slice(last, m.index));
      breaks.push(m);
      last = m.index + m[0].length;
    }
    lines.push(raw.slice(last));
    const isFence = lines.map((l) => FENCE_LINE.test(l));
    const hasFence = isFence.some(Boolean);
    const fences: string[] = [];
    let run: number | null = null;
    const closeRun = () => {
      if (run !== null) this.close(run);
      run = null;
    };
    lines.forEach((line, i) => {
      if (i > 0) {
        const brk = breaks[i - 1];
        if (isFence[i] || isFence[i - 1]) {
          // Line breaks around fence lines are structure and must stay where they are.
          closeRun();
          fences.push(`x${this.placeholder(brk[0], 'line break', true)}`);
        } else {
          if (!this.softBreak) this.softBreak = { eol: brk[1], prefix: brk[2] };
          this.text(' ');
        }
      }
      if (isFence[i]) {
        closeRun();
        fences.push(...this.fenceLine(line, mode));
      } else {
        // Anchor each text run between fence lines in a pair so moving text across fences is detectable.
        if (hasFence && run === null && line) {
          run = this.open('html', '', '', 'text between directive lines');
          fences.push(`g${run}`);
        }
        this.sourceLine(line, mode);
      }
    });
    closeRun();
    if (fences.length > 1) this.tagGroups.push({ tokens: fences, contiguous: false });
  }

  /** `:::name Title` keeps the fence and translates the title; `:::image alt-text="..." :::` translates only prose attributes. */
  private fenceLine(line: string, mode: EscapeMode): string[] {
    if (!line.startsWith(':::') || /\w[\w-]*\s*=\s*["']/.test(line)) return this.attributed(line);
    const m = /^(:{3,}\s*[A-Za-z][\w-]*\s*)(.*)$/.exec(line);
    if (m && /\p{L}/u.test(m[2]) && !/:{3,}\s*$/.test(m[2])) {
      const x = this.placeholder(m[1], m[1]);
      const g = this.open('html', '', '', 'directive title');
      this.sourceLine(m[2], mode);
      this.close(g);
      return [`x${x}`, `g${g}`];
    }
    return [`x${this.placeholder(line, line)}`];
  }

  /** Protected token with translatable attribute values (Hugo shortcodes, Docs directives). Returns the tag ids. */
  attributed(raw: string): string[] {
    const values = [...raw.matchAll(ATTR_VALUE)].filter((v) => /\p{L}/u.test(v[4]));
    if (!values.length) return [`x${this.placeholder(raw, raw)}`];
    const tokens: string[] = [];
    let pos = 0;
    for (const v of values) {
      const start = v.index! + v[1].length + v[2].length + 1;
      tokens.push(`x${this.placeholder(raw.slice(pos, start), raw.slice(pos, start))}`);
      const g = this.open('html', '', '', `${v[1]} attribute value`);
      this.pairs.get(g)!.quote = v[3];
      tokens.push(`g${g}`);
      this.sourceLine(v[4], 'none');
      this.close(g);
      pos = start + v[4].length;
    }
    tokens.push(`x${this.placeholder(raw.slice(pos), raw.slice(pos))}`);
    this.tagGroups.push({ tokens, contiguous: true });
    return tokens;
  }

  private sourceLine(raw: string, mode: EscapeMode) {
    if (!raw) return;
    const spans = findProtected(raw, this.dnt).map((s) => ({ ...s, kind: 'protect' as const }));
    const extra: { start: number; end: number; kind: 'escape' | 'entity' }[] = [];
    if (mode === 'markdown') for (const m of raw.matchAll(MD_ESCAPE)) extra.push({ start: m.index!, end: m.index! + m[0].length, kind: 'escape' });
    if (mode !== 'none') for (const m of raw.matchAll(ENTITY)) extra.push({ start: m.index!, end: m.index! + m[0].length, kind: 'entity' });
    // Escapes inside a protected span stay part of that span.
    const all = [...spans, ...extra.filter((e) => !spans.some((s) => e.start >= s.start && e.end <= s.end))].sort(
      (a, b) => a.start - b.start || b.end - a.end,
    );
    let pos = 0;
    for (const s of all) {
      if (s.start < pos) continue;
      this.text(raw.slice(pos, s.start));
      const bytes = raw.slice(s.start, s.end);
      if ('rule' in s && s.rule === 'template' && /^\{\{[<%]/.test(bytes)) this.attributed(bytes);
      else this.placeholder(bytes, s.kind === 'escape' ? bytes.slice(1) : bytes);
      pos = s.end;
    }
    this.text(raw.slice(pos));
  }
}

function truncate(s: string): string {
  const one = s.replace(/\s+/g, ' ');
  return one.length > 80 ? one.slice(0, 77) + '...' : one;
}

export type MaskToken =
  | { t: 'text'; v: string }
  | { t: 'x'; n: number }
  | { t: 'open'; n: number }
  | { t: 'close'; n: number };

const TAG = /<(\/?)([gx])(\d+)\s*(\/?)>/g;

export function tokenize(masked: string): MaskToken[] {
  const out: MaskToken[] = [];
  let last = 0;
  TAG.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = TAG.exec(masked))) {
    if (m.index > last) out.push({ t: 'text', v: masked.slice(last, m.index) });
    const n = Number(m[3]);
    if (m[2] === 'x') out.push({ t: 'x', n });
    else if (m[1] === '/') out.push({ t: 'close', n });
    else if (m[4] === '/') out.push({ t: 'x', n: -n });
    else out.push({ t: 'open', n });
    last = m.index + m[0].length;
  }
  if (last < masked.length) out.push({ t: 'text', v: masked.slice(last) });
  return out;
}

/** Structural tag check: every placeholder once, every pair opened and closed once and properly nested. */
export function checkTags(seg: Segment, translated: string): string[] {
  const errors: string[] = [];
  const tokens = tokenize(translated);
  const seenX = new Map<number, number>();
  const seenOpen = new Set<number>();
  const seenClose = new Set<number>();
  const stack: number[] = [];
  for (const tok of tokens) {
    if (tok.t === 'x') {
      if (tok.n < 0 || !seg.placeholders.has(tok.n)) {
        errors.push(`unknown tag <x${Math.abs(tok.n)}/>`);
        continue;
      }
      seenX.set(tok.n, (seenX.get(tok.n) ?? 0) + 1);
    } else if (tok.t === 'open') {
      if (!seg.pairs.has(tok.n)) errors.push(`unknown tag <g${tok.n}>`);
      else if (seenOpen.has(tok.n)) errors.push(`<g${tok.n}> used more than once`);
      seenOpen.add(tok.n);
      stack.push(tok.n);
    } else if (tok.t === 'close') {
      if (!seg.pairs.has(tok.n)) errors.push(`unknown tag </g${tok.n}>`);
      if (stack[stack.length - 1] !== tok.n) errors.push(`</g${tok.n}> closes out of order`);
      else stack.pop();
      seenClose.add(tok.n);
    }
  }
  for (const n of seg.placeholders.keys()) {
    const c = seenX.get(n) ?? 0;
    if (c !== 1) errors.push(c === 0 ? `missing <x${n}/>` : `<x${n}/> used ${c} times`);
  }
  for (const n of seg.pairs.keys()) {
    if (!seenOpen.has(n) || !seenClose.has(n)) errors.push(`pair <g${n}>…</g${n}> missing`);
  }
  if (stack.length) errors.push(`unclosed tags: ${stack.map((n) => `<g${n}>`).join(', ')}`);
  errors.push(...checkTagGroups(seg, tokens));
  return [...new Set(errors)];
}

const tokenKey = (t: MaskToken) => (t.t === 'x' ? `x${t.n}` : t.t === 'open' ? `g${t.n}` : t.t === 'close' ? `/g${t.n}` : '');

/** Order constraints for fence lines and attribute values; quotes must not leak into attribute values. */
function checkTagGroups(seg: Segment, tokens: MaskToken[]): string[] {
  const errors: string[] = [];
  const inside: number[] = [];
  for (const t of tokens) {
    if (t.t === 'open') inside.push(t.n);
    else if (t.t === 'close') inside.pop();
    else if (t.t === 'text') {
      for (const n of inside) {
        const q = seg.pairs.get(n)?.quote;
        if (q && xmlUnescape(t.v).includes(q)) errors.push(`text inside <g${n}> must not contain ${q}`);
      }
    }
  }
  for (const group of seg.tagGroups ?? []) {
    const keys = tokens.map(tokenKey);
    const positions = group.tokens.map((k) => keys.indexOf(k));
    if (positions.some((p, i) => p === -1 || (i > 0 && p <= positions[i - 1]))) {
      errors.push(`keep ${group.tokens.map((k) => `<${k}${k.startsWith('x') ? '/' : ''}>`).join(' ')} in their original order`);
      continue;
    }
    if (!group.contiguous) continue;
    // Contiguous: nothing but the attribute text (inside <gN>…</gN>) may appear between the group's tags.
    let i = positions[0];
    let ok = true;
    for (const k of group.tokens) {
      if (tokenKey(tokens[i]) !== k) {
        ok = false;
        break;
      }
      i = k.startsWith('g') ? keys.indexOf(`/${k}`, i) + 1 : i + 1;
    }
    if (!ok) {
      errors.push(`do not move text into or out of the attribute values ${group.tokens.filter((k) => k.startsWith('g')).map((k) => `<${k}>`).join(', ')}`);
    }
  }
  return errors;
}

export function plainText(masked: string): string {
  return tokenize(masked)
    .filter((t) => t.t === 'text')
    .map((t) => xmlUnescape((t as { v: string }).v))
    .join('');
}

const MD_SPECIAL = ['\\', '`', '*', '_', '[', ']', '<', '>', '~', '|', '&', '#', '!'];

const BARE_AMP = /&(?!(?:[A-Za-z][A-Za-z0-9]{1,31}|#\d{1,7}|#[xX][0-9a-fA-F]{1,6});)/g;

/**
 * Builds the escaper for translated text. For Markdown only characters the translation introduced
 * (more occurrences than in the source) are escaped, so untouched literal characters keep their bytes.
 */
export function makeEscaper(ctx: TextContext, translatedText: string, sourceText: string, quote?: string): (s: string) => string {
  switch (ctx) {
    case 'markdown':
    case 'cell': {
      const chars = MD_SPECIAL.filter((ch) => (ctx === 'cell' && ch === '|') || count(translatedText, ch) > count(sourceText, ch));
      if (!chars.length) return (s) => s;
      return (s) => {
        let out = s;
        for (const ch of chars) out = out.split(ch).join('\\' + ch);
        return out;
      };
    }
    case 'html':
      return (s) => s.replace(BARE_AMP, '&amp;').replace(/</g, '&lt;');
    case 'attr':
      return (s) => {
        let out = s.replace(BARE_AMP, '&amp;').replace(/</g, '&lt;');
        if (quote === '"') out = out.replace(/"/g, '&quot;');
        if (quote === "'") out = out.replace(/'/g, '&#39;');
        return out;
      };
    case 'mdtitle':
      if (quote === '(') return (s) => s.replace(/[()]/g, '\\$&');
      return (s) => (quote ? s.split(quote).join('\\' + quote) : s);
    case 'comment':
    case 'yaml':
      return (s) => s;
  }
}

function count(s: string, ch: string): number {
  let c = 0;
  for (let i = s.indexOf(ch); i !== -1; i = s.indexOf(ch, i + 1)) c++;
  return c;
}

export interface Piece {
  s: string;
  atom: boolean;
  hardBreak?: boolean;
}

/** Converts a translated masked string into output pieces (text is unescaped, then escaped for its context). */
export function renderPieces(seg: Segment, translated: string, tm: TMap, sourceText: string): Piece[] {
  const tokens = normalizeEmphasisWhitespace(seg, tokenize(translated));
  const pieces: Piece[] = [];
  const texts = tokens.filter((t) => t.t === 'text').map((t) => xmlUnescape((t as { v: string }).v));
  const escape = makeEscaper(seg.textContext, texts.join(''), sourceText, seg.quote);
  let ti = 0;
  const openAt = new Map<number, number>();
  for (const tok of tokens) {
    if (tok.t === 'text') pieces.push({ s: escape(texts[ti++]), atom: false });
    else if (tok.t === 'x') {
      const p = seg.placeholders.get(tok.n)!;
      pieces.push({ s: p.render(tm), atom: true, hardBreak: p.hardBreak });
    } else if (tok.t === 'open') {
      openAt.set(tok.n, pieces.length);
      pieces.push({ s: seg.pairs.get(tok.n)!.open(tm), atom: true });
    } else {
      const pair = seg.pairs.get(tok.n)!;
      const inner = pieces.slice((openAt.get(tok.n) ?? pieces.length) + 1).map((p) => p.s).join('');
      const unchangedRef = pair.refLabel !== undefined && inner === pair.refLabel;
      pieces.push({ s: unchangedRef ? pair.closeOriginal! : pair.close(tm), atom: true });
    }
  }
  return pieces;
}

/** Emphasis delimiters must hug their content: "<g1> text</g1>" becomes " <g1>text</g1>". */
function normalizeEmphasisWhitespace(seg: Segment, tokens: MaskToken[]): MaskToken[] {
  const out = tokens.map((t) => ({ ...t }));
  const isEm = (n: number) => {
    const k = seg.pairs.get(n)?.kind;
    return k === 'emphasis' || k === 'strong' || k === 'delete';
  };
  for (let i = 0; i < out.length; i++) {
    const tok = out[i];
    if (tok.t === 'open' && isEm(tok.n)) {
      const next = out[i + 1];
      if (next?.t === 'text') {
        const ws = next.v.match(/^\s+/)?.[0];
        if (ws) {
          next.v = next.v.slice(ws.length);
          const prev = out[i - 1];
          if (prev?.t === 'text') prev.v += ws;
          else out.splice(i, 0, { t: 'text', v: ws }), i++;
        }
      }
    } else if (tok.t === 'close' && isEm(tok.n)) {
      const prev = out[i - 1];
      if (prev?.t === 'text') {
        const ws = prev.v.match(/\s+$/)?.[0];
        if (ws) {
          prev.v = prev.v.slice(0, -ws.length);
          const next = out[i + 1];
          if (next?.t === 'text') next.v = ws + next.v;
          else out.splice(i + 1, 0, { t: 'text', v: ws });
        }
      }
    }
  }
  return out.filter((t) => t.t !== 'text' || t.v !== '');
}
