import { MaskBuilder } from '../mask/masking.js';
import { renderSegment } from './render.js';
import { TRANSLATABLE_ATTRS, parseAttributes } from './htmlAttrs.js';
import type { RenderOptions, Replacement, Segment, SegmentKind, TMap, TextContext, WrapSpec } from './types.js';
import type { ParseOptions } from './parse.js';

export interface SegmentOptions {
  kind: SegmentKind;
  textContext: TextContext;
  original: string;
  note: string;
  wrap?: WrapSpec;
  forbidden?: string[];
  quote?: string;
  finalize?: (s: string) => string;
  embedded?: boolean;
  inlineSignature?: string;
  dependents?: string[];
  /** Create the segment even without own text when it has dependents (it then becomes passive). */
  force?: boolean;
}

export interface ExtractOptions {
  parse?: ParseOptions;
  /** Translate Python docstrings (off by default; they are string literals, not comments). */
  docstrings?: boolean;
}

export class ExtractContext {
  readonly segments: Segment[] = [];
  readonly byId = new Map<string, Segment>();
  readonly replacements: Replacement[] = [];
  readonly notes: string[] = [];
  /** Structural position applied to segments created from now on. */
  structure: string | undefined;
  private counter = 0;

  constructor(
    readonly source: string,
    readonly dnt: readonly string[],
    readonly eol: string,
    readonly options: ExtractOptions = {},
  ) {}

  builder(): MaskBuilder {
    return new MaskBuilder(this.dnt);
  }

  /** Creates a segment when the builder holds translatable text, otherwise returns null. */
  segment(mb: MaskBuilder, o: SegmentOptions): Segment | null {
    const hasText = /\p{L}/u.test(mb.sourceText);
    if (!hasText && !(o.force && o.dependents?.length)) return null;
    const seg: Segment = {
      id: `s${++this.counter}`,
      kind: o.kind,
      masked: mb.masked,
      placeholders: mb.placeholders,
      pairs: mb.pairs,
      textContext: o.textContext,
      original: o.original,
      sourceText: mb.sourceText,
      note: o.note,
      structure: this.structure,
      tagGroups: mb.tagGroups.length ? mb.tagGroups : undefined,
      wrap: o.wrap,
      forbidden: o.forbidden,
      quote: o.quote,
      finalize: o.finalize,
      embedded: o.embedded,
      inlineSignature: o.inlineSignature,
      dependents: o.dependents?.length ? o.dependents : undefined,
      passive: !hasText || undefined,
    };
    this.segments.push(seg);
    this.byId.set(seg.id, seg);
    return seg;
  }

  render(seg: Segment, tm: TMap, opts?: RenderOptions): string {
    return renderSegment(seg, tm, this.byId, opts);
  }

  replace(start: number, end: number, seg: Segment) {
    this.replacements.push({ start, end, render: (tm, opts) => this.render(seg, tm, opts), segmentIds: [seg.id] });
  }

  /**
   * Makes translatable attributes (alt, title, aria-label...) of a single tag into embedded segments.
   * Returns a renderer producing the tag with translated values, plus the embedded segment ids.
   */
  tagWithAttributes(tag: string, note: string): { render: (tm: TMap) => string; ids: string[] } {
    const parts: { start: number; end: number; seg: Segment }[] = [];
    for (const a of parseAttributes(tag)) {
      if (!TRANSLATABLE_ATTRS.has(a.name)) continue;
      const value = tag.slice(a.start, a.end);
      const mb = this.builder();
      mb.source(value, 'html');
      const seg = this.segment(mb, {
        kind: a.name === 'alt' ? 'alt' : 'attr',
        textContext: 'attr',
        original: value,
        note: `${note}, HTML ${a.name} attribute`,
        quote: a.quote,
        embedded: true,
        finalize: a.quote ? undefined : (s) => (/^[^\s"'=<>`]+$/.test(s) ? s : `"${s.replace(/"/g, '&quot;')}"`),
      });
      if (seg) parts.push({ start: a.start, end: a.end, seg });
    }
    if (!parts.length) return { render: () => tag, ids: [] };
    return {
      ids: parts.map((p) => p.seg.id),
      render: (tm) => {
        let out = '';
        let pos = 0;
        for (const p of parts) {
          out += tag.slice(pos, p.start) + this.render(p.seg, tm);
          pos = p.end;
        }
        return out + tag.slice(pos);
      },
    };
  }
}
