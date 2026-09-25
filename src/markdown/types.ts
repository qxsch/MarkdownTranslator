/** Translated masked text per segment id; a missing entry means "keep source". */
export type TMap = ReadonlyMap<string, string>;

export type SegmentKind =
  | 'paragraph'
  | 'heading'
  | 'cell'
  | 'alt'
  | 'title'
  | 'attr'
  | 'html'
  | 'comment'
  | 'frontmatter';

export type TextContext = 'markdown' | 'cell' | 'html' | 'attr' | 'mdtitle' | 'comment' | 'yaml';

export interface Placeholder {
  n: number;
  /** Short preview of what the placeholder stands for, shown to the model for grammar context. */
  hint: string;
  render: (tm: TMap) => string;
  /** True when the rendered bytes contain a line break (hard break), relevant for wrapping. */
  hardBreak?: boolean;
}

export type PairKind = 'emphasis' | 'strong' | 'delete' | 'link' | 'image' | 'html';

export interface Pair {
  n: number;
  kind: PairKind;
  hint: string;
  open: (tm: TMap) => string;
  close: (tm: TMap) => string;
  /** For [label] / [label][] references: original label and closing bytes, kept when the label is unchanged. */
  refLabel?: string;
  closeOriginal?: string;
}

export interface WrapSpec {
  /** Maximum line width observed in the source block. */
  width: number;
  /** Column where the first line of the block starts. */
  firstColumn: number;
  /** Bytes emitted after each inserted line break (container prefix / indentation). */
  prefix: string;
  eol: string;
}

export interface Segment {
  id: string;
  kind: SegmentKind;
  /** Source text with inline structure replaced by <gN>…</gN> and <xN/> tags; text is XML-escaped. */
  masked: string;
  placeholders: Map<number, Placeholder>;
  pairs: Map<number, Pair>;
  textContext: TextContext;
  /** Original bytes of the segment; returned verbatim when the segment is untranslated. */
  original: string;
  /** Plain source text outside of tags, used to decide which markup characters need escaping. */
  sourceText: string;
  /** Human-readable context passed to the model (e.g. "table cell", "python comment"). */
  note: string;
  wrap?: WrapSpec;
  /** Substrings that must not appear in translated text (e.g. comment terminators). */
  forbidden?: string[];
  /** Quote character for attribute/title contexts. */
  quote?: string;
  /** Final transformation over the rendered text (e.g. YAML quoting). */
  finalize?: (rendered: string) => string;
  /** Inline signature of the source (for prose kinds) used for structural validation. */
  inlineSignature?: string;
  /** Segments that are rendered inside another segment's tag instead of at their own range. */
  embedded?: boolean;
  /** Ids of embedded segments rendered inside this segment. */
  dependents?: string[];
  /** Passive segments carry no translatable text of their own and are never sent to a translator. */
  passive?: boolean;
}

export interface RenderOptions {
  wrap?: boolean;
  /** Render through the placeholder path even for untranslated segments (tests byte fidelity). */
  force?: boolean;
}

export interface Replacement {
  start: number;
  end: number;
  render: (tm: TMap, opts?: RenderOptions) => string;
  segmentIds: string[];
}

export interface HeadingInfo {
  /** Offset of the first content character of the heading in the source. */
  contentStart: number;
  hasExplicitId: boolean;
}

export interface Extraction {
  source: string;
  bom: string;
  eol: string;
  segments: Segment[];
  byId: ReadonlyMap<string, Segment>;
  replacements: Replacement[];
  notes: string[];
  /** Link/footnote definitions, appended when validating isolated inline content. */
  definitionsText: string;
  frontmatterFormality?: 'formal' | 'informal';
}
